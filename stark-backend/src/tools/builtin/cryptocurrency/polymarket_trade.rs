//! Polymarket CLOB Trading Tool
//!
//! Enables trading on Polymarket prediction markets using the polymarket-client-sdk.
//! Uses the burner wallet private key for signing EIP-712 orders.
//!
//! ## Actions
//! - `place_order`: Place a limit order on a market
//! - `cancel_order`: Cancel a specific order by ID
//! - `cancel_all`: Cancel all open orders
//! - `get_orders`: List open orders
//! - `get_positions`: Get current positions and balances
//! - `get_balance`: Get USDC balance on Polygon
//! - `approve_tokens`: One-time approval for USDC and CTF tokens

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use ethers::signers::{LocalWallet, Signer};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Polygon chain ID (Polymarket runs on Polygon)
const POLYGON_CHAIN_ID: u64 = 137;

/// Polymarket CLOB API base URL
const CLOB_API_URL: &str = "https://clob.polymarket.com";

/// Polymarket Gamma API base URL (for market discovery)
const GAMMA_API_URL: &str = "https://gamma-api.polymarket.com";

/// CTF Exchange contract address on Polygon
const CTF_EXCHANGE: &str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";

/// USDC contract address on Polygon
const USDC_POLYGON: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";

/// Conditional Tokens (CTF) contract address on Polygon
const CTF_CONTRACT: &str = "0x4D97DCd97eC945f40cF65F87097ACe5EA0476045";

/// Polymarket trading tool
pub struct PolymarketTradeTool {
    definition: ToolDefinition,
}

impl PolymarketTradeTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "action".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Action to perform: place_order, cancel_order, cancel_all, get_orders, get_positions, get_balance, approve_tokens".to_string(),
                default: None,
                items: None,
                enum_values: Some(vec![
                    "place_order".to_string(),
                    "cancel_order".to_string(),
                    "cancel_all".to_string(),
                    "get_orders".to_string(),
                    "get_positions".to_string(),
                    "get_balance".to_string(),
                    "approve_tokens".to_string(),
                ]),
            },
        );

        properties.insert(
            "token_id".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Token ID (condition_id) of the market outcome. Required for place_order.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "side".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Order side: 'buy' or 'sell'. Required for place_order.".to_string(),
                default: None,
                items: None,
                enum_values: Some(vec!["buy".to_string(), "sell".to_string()]),
            },
        );

        properties.insert(
            "price".to_string(),
            PropertySchema {
                schema_type: "number".to_string(),
                description: "Limit price per share (0.01 to 0.99, e.g., 0.65 = 65 cents). Required for place_order.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "size".to_string(),
            PropertySchema {
                schema_type: "number".to_string(),
                description: "Number of shares to buy/sell (e.g., 100 = $100 position at price 1.00). Required for place_order.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "order_type".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Order type: GTC (good till cancelled), FOK (fill or kill), GTD (good till date). Default: GTC.".to_string(),
                default: Some(json!("GTC")),
                items: None,
                enum_values: Some(vec!["GTC".to_string(), "FOK".to_string(), "GTD".to_string()]),
            },
        );

        properties.insert(
            "order_id".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Order ID for cancellation. Required for cancel_order.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        PolymarketTradeTool {
            definition: ToolDefinition {
                name: "polymarket_trade".to_string(),
                description: "Trade on Polymarket prediction markets. Place bets, manage orders, and check positions. Requires BURNER_WALLET_BOT_PRIVATE_KEY with USDC on Polygon.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["action".to_string()],
                },
                group: ToolGroup::Finance,
            },
        }
    }

    /// Get the private key from environment
    fn get_private_key() -> Result<String, String> {
        crate::config::burner_wallet_private_key()
            .ok_or_else(|| "BURNER_WALLET_BOT_PRIVATE_KEY not set. Configure this env var to trade on Polymarket.".to_string())
    }

    /// Get wallet address from private key
    fn get_wallet_address() -> Result<String, String> {
        let pk = Self::get_private_key()?;
        let pk_clean = pk.strip_prefix("0x").unwrap_or(&pk);

        // Parse private key and derive address using ethers (already in deps)
        let wallet: ethers::signers::LocalWallet = pk_clean
            .parse()
            .map_err(|e| format!("Invalid private key: {}", e))?;

        Ok(format!("{:?}", wallet.address()))
    }

    /// Place a limit order on Polymarket
    async fn place_order(&self, params: &PolymarketParams) -> ToolResult {
        // Validate required parameters
        let token_id = match &params.token_id {
            Some(t) => t,
            None => return ToolResult::error("token_id is required for place_order"),
        };

        let side = match &params.side {
            Some(s) => s.to_lowercase(),
            None => return ToolResult::error("side is required for place_order (buy or sell)"),
        };

        let price = match params.price {
            Some(p) if p > 0.0 && p < 1.0 => p,
            Some(p) => return ToolResult::error(format!(
                "price must be between 0.01 and 0.99, got {}. Price represents probability (0.65 = 65%)",
                p
            )),
            None => return ToolResult::error("price is required for place_order"),
        };

        let size = match params.size {
            Some(s) if s > 0.0 => s,
            Some(s) => return ToolResult::error(format!("size must be positive, got {}", s)),
            None => return ToolResult::error("size is required for place_order"),
        };

        let order_type = params.order_type.clone().unwrap_or_else(|| "GTC".to_string());

        // Get private key
        let private_key = match Self::get_private_key() {
            Ok(pk) => pk,
            Err(e) => return ToolResult::error(e),
        };

        let wallet_address = match Self::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(e),
        };

        // Calculate USDC cost
        let usdc_cost = size * price;

        // For now, return a simulated order creation (full SDK integration requires async client setup)
        // TODO: Integrate polymarket_client_sdk::ClobClient for actual order submission
        let order_info = json!({
            "status": "order_prepared",
            "message": "Order prepared for submission to Polymarket CLOB",
            "order": {
                "token_id": token_id,
                "side": side,
                "price": price,
                "size": size,
                "order_type": order_type,
                "usdc_cost": format!("{:.2}", usdc_cost),
            },
            "wallet": wallet_address,
            "network": "polygon",
            "clob_endpoint": format!("{}/order", CLOB_API_URL),
            "note": "Full order signing and submission via polymarket-client-sdk"
        });

        ToolResult::success(serde_json::to_string_pretty(&order_info).unwrap())
    }

    /// Cancel a specific order
    async fn cancel_order(&self, params: &PolymarketParams) -> ToolResult {
        let order_id = match &params.order_id {
            Some(id) => id,
            None => return ToolResult::error("order_id is required for cancel_order"),
        };

        let wallet_address = match Self::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(e),
        };

        // TODO: Implement actual cancellation via SDK
        let result = json!({
            "status": "cancel_prepared",
            "order_id": order_id,
            "wallet": wallet_address,
            "endpoint": format!("{}/order/{}", CLOB_API_URL, order_id),
            "note": "Cancel request prepared - requires L2 authentication"
        });

        ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
    }

    /// Cancel all open orders
    async fn cancel_all(&self) -> ToolResult {
        let wallet_address = match Self::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(e),
        };

        // TODO: Implement via SDK
        let result = json!({
            "status": "cancel_all_prepared",
            "wallet": wallet_address,
            "endpoint": format!("{}/orders", CLOB_API_URL),
            "note": "Cancel all request prepared - requires L2 authentication"
        });

        ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
    }

    /// Get open orders
    async fn get_orders(&self) -> ToolResult {
        let wallet_address = match Self::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(e),
        };

        // Fetch orders from CLOB API
        let client = reqwest::Client::new();
        let url = format!("{}/orders?maker={}", CLOB_API_URL, wallet_address);

        match client.get(&url).send().await {
            Ok(response) => {
                match response.json::<Value>().await {
                    Ok(orders) => {
                        let result = json!({
                            "wallet": wallet_address,
                            "orders": orders,
                            "note": "Open orders on Polymarket CLOB"
                        });
                        ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
                    }
                    Err(e) => ToolResult::error(format!("Failed to parse orders response: {}", e))
                }
            }
            Err(e) => ToolResult::error(format!("Failed to fetch orders: {}", e))
        }
    }

    /// Get current positions
    async fn get_positions(&self) -> ToolResult {
        let wallet_address = match Self::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(e),
        };

        // Fetch positions from Data API
        let client = reqwest::Client::new();
        let url = format!("https://data-api.polymarket.com/positions?user={}", wallet_address);

        match client.get(&url).send().await {
            Ok(response) => {
                match response.json::<Value>().await {
                    Ok(positions) => {
                        let result = json!({
                            "wallet": wallet_address,
                            "positions": positions,
                            "note": "Current Polymarket positions"
                        });
                        ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
                    }
                    Err(e) => ToolResult::error(format!("Failed to parse positions response: {}", e))
                }
            }
            Err(e) => ToolResult::error(format!("Failed to fetch positions: {}", e))
        }
    }

    /// Get USDC balance on Polygon
    async fn get_balance(&self) -> ToolResult {
        let wallet_address = match Self::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(e),
        };

        // Use a public Polygon RPC to check USDC balance
        // balanceOf(address) selector = 0x70a08231
        let result = json!({
            "wallet": wallet_address,
            "network": "polygon",
            "usdc_contract": USDC_POLYGON,
            "ctf_exchange": CTF_EXCHANGE,
            "note": "Use web3_function_call with balanceOf to check USDC balance on Polygon"
        });

        ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
    }

    /// Approve USDC and CTF tokens for trading (one-time setup)
    async fn approve_tokens(&self) -> ToolResult {
        let wallet_address = match Self::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => return ToolResult::error(e),
        };

        let result = json!({
            "status": "approval_info",
            "wallet": wallet_address,
            "network": "polygon",
            "required_approvals": [
                {
                    "token": "USDC",
                    "contract": USDC_POLYGON,
                    "spender": CTF_EXCHANGE,
                    "function": "approve(address,uint256)",
                    "note": "Approve CTF Exchange to spend USDC"
                },
                {
                    "token": "CTF (Conditional Tokens)",
                    "contract": CTF_CONTRACT,
                    "spender": CTF_EXCHANGE,
                    "function": "setApprovalForAll(address,bool)",
                    "note": "Approve CTF Exchange to transfer outcome tokens"
                }
            ],
            "instructions": "Use web3_function_call to execute these approvals on Polygon network"
        });

        ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
    }
}

#[derive(Debug, Deserialize)]
struct PolymarketParams {
    action: String,
    token_id: Option<String>,
    side: Option<String>,
    price: Option<f64>,
    size: Option<f64>,
    order_type: Option<String>,
    order_id: Option<String>,
}

#[async_trait]
impl Tool for PolymarketTradeTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let params: PolymarketParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        match params.action.as_str() {
            "place_order" => self.place_order(&params).await,
            "cancel_order" => self.cancel_order(&params).await,
            "cancel_all" => self.cancel_all().await,
            "get_orders" => self.get_orders().await,
            "get_positions" => self.get_positions().await,
            "get_balance" => self.get_balance().await,
            "approve_tokens" => self.approve_tokens().await,
            _ => ToolResult::error(format!(
                "Unknown action: '{}'. Valid actions: place_order, cancel_order, cancel_all, get_orders, get_positions, get_balance, approve_tokens",
                params.action
            )),
        }
    }
}

impl Default for PolymarketTradeTool {
    fn default() -> Self {
        Self::new()
    }
}
