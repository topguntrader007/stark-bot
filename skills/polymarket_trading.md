---
name: polymarket_trading
description: "Trade on Polymarket prediction markets - place bets, manage orders, check positions and P&L."
version: 1.0.0
author: starkbot
homepage: https://docs.polymarket.com/
metadata: {"clawdbot":{"emoji":"🎲"}}
requires_tools: [polymarket_trade, web_fetch]
tags: [polymarket, prediction-markets, trading, betting, crypto, defi, polygon]
arguments:
  action:
    description: "Trading action: bet, positions, orders, cancel"
    required: false
  market:
    description: "Market question or token ID to trade"
    required: false
  amount:
    description: "Amount in USD to bet"
    required: false
  outcome:
    description: "Outcome to bet on (YES/NO or specific outcome name)"
    required: false
---

# Polymarket Trading Guide

You can trade on Polymarket prediction markets using the `polymarket_trade` tool. This allows placing bets, managing orders, and tracking positions.

## Prerequisites

1. **Wallet Setup**: `BURNER_WALLET_BOT_PRIVATE_KEY` must be configured
2. **USDC on Polygon**: The wallet needs USDC on Polygon network for betting
3. **Token Approvals**: One-time approval needed (use `approve_tokens` action)

---

## Trading Workflow

### Step 1: Find a Market

First, use the `polymarket` skill or `web_fetch` to find markets:

```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events?active=true&limit=10",
  "extract_mode": "raw"
}
```

Or search by topic:
```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events?active=true&_q=bitcoin",
  "extract_mode": "raw"
}
```

### Step 2: Get Market Details & Token ID

From the market data, extract:
- **token_id**: The `conditionId` or outcome token ID
- **Current price**: Use CLOB API to get best bid/ask

```json
{
  "tool": "web_fetch",
  "url": "https://clob.polymarket.com/book?token_id=<TOKEN_ID>",
  "extract_mode": "raw"
}
```

### Step 3: Check Spread & Liquidity

Before placing an order, check the orderbook spread:

```json
{
  "tool": "web_fetch",
  "url": "https://clob.polymarket.com/spread?token_id=<TOKEN_ID>",
  "extract_mode": "raw"
}
```

### Step 4: Place Order

Use the `polymarket_trade` tool:

```json
{
  "tool": "polymarket_trade",
  "action": "place_order",
  "token_id": "0x...",
  "side": "buy",
  "price": 0.65,
  "size": 100,
  "order_type": "GTC"
}
```

**Parameters:**
- `token_id`: The outcome token to trade
- `side`: `buy` (bet YES) or `sell` (bet NO / exit position)
- `price`: Limit price 0.01-0.99 (0.65 = 65 cents = 65% implied probability)
- `size`: Number of shares (100 shares @ $0.65 = $65 cost)
- `order_type`: `GTC` (good till cancelled), `FOK` (fill or kill), `GTD` (good till date)

---

## Tool Actions Reference

### Place Order
```json
{
  "tool": "polymarket_trade",
  "action": "place_order",
  "token_id": "0x...",
  "side": "buy",
  "price": 0.55,
  "size": 50
}
```

### Get Open Orders
```json
{
  "tool": "polymarket_trade",
  "action": "get_orders"
}
```

### Cancel Specific Order
```json
{
  "tool": "polymarket_trade",
  "action": "cancel_order",
  "order_id": "order-uuid-here"
}
```

### Cancel All Orders
```json
{
  "tool": "polymarket_trade",
  "action": "cancel_all"
}
```

### Get Current Positions
```json
{
  "tool": "polymarket_trade",
  "action": "get_positions"
}
```

### Get Balance Info
```json
{
  "tool": "polymarket_trade",
  "action": "get_balance"
}
```

### Setup Token Approvals (One-Time)
```json
{
  "tool": "polymarket_trade",
  "action": "approve_tokens"
}
```

---

## Understanding Prices & Outcomes

### Binary Markets (YES/NO)
- Price represents implied probability
- 0.65 price = 65% implied chance of YES
- YES + NO prices should sum to ~1.00
- Buy YES if you think probability > current price
- Buy NO if you think probability < current price

### Multi-Outcome Markets
- Each outcome has its own token
- Prices represent relative probabilities
- Sum of all outcomes ~1.00

### Calculating Costs & Payouts

| Action | Cost | Max Payout |
|--------|------|------------|
| Buy 100 YES @ $0.65 | $65 | $100 (if YES wins) |
| Buy 100 NO @ $0.35 | $35 | $100 (if NO wins) |

**Profit if correct:** `(1.00 - price) × size`
**Loss if wrong:** `price × size`

---

## Risk Management Rules

1. **Check Spread First**: Wide spreads (>5%) indicate low liquidity
2. **Use Limit Orders**: Avoid market orders to prevent slippage
3. **Position Sizing**: Never bet more than you can afford to lose
4. **Verify Token ID**: Double-check you're trading the right outcome
5. **Monitor Positions**: Check `get_positions` regularly

---

## Example Trading Session

### User: "Bet $20 on YES for the Bitcoin $100k market"

**Agent Workflow:**

1. **Search for market:**
```json
{"tool": "web_fetch", "url": "https://gamma-api.polymarket.com/events?_q=bitcoin+100k&active=true"}
```

2. **Get current price:**
```json
{"tool": "web_fetch", "url": "https://clob.polymarket.com/midpoint?token_id=<TOKEN_ID>"}
```

3. **Check spread:**
```json
{"tool": "web_fetch", "url": "https://clob.polymarket.com/spread?token_id=<TOKEN_ID>"}
```

4. **Place order** (if spread acceptable):
```json
{
  "tool": "polymarket_trade",
  "action": "place_order",
  "token_id": "<TOKEN_ID>",
  "side": "buy",
  "price": 0.45,
  "size": 44
}
```
(44 shares × $0.45 = ~$20)

5. **Confirm to user:**
"Placed order to buy 44 YES shares at $0.45 (45% implied probability). If Bitcoin hits $100k, you'll receive $44 profit. Order ID: xxx"

---

## Contract Addresses (Polygon)

| Contract | Address |
|----------|---------|
| CTF Exchange | `0xC5d563A36AE78145C45a50134d48A1215220f80a` |
| USDC | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` |
| Conditional Tokens | `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` |

---

## API Endpoints

| API | Base URL | Purpose |
|-----|----------|---------|
| Gamma | `https://gamma-api.polymarket.com` | Market discovery |
| CLOB | `https://clob.polymarket.com` | Prices, orders, trading |
| Data | `https://data-api.polymarket.com` | Positions, history |

---

## Error Handling

| Error | Cause | Solution |
|-------|-------|----------|
| "Insufficient balance" | Not enough USDC | Bridge USDC to Polygon |
| "Token not approved" | Missing approval | Run `approve_tokens` action |
| "Invalid price" | Price outside 0.01-0.99 | Use valid probability price |
| "Order rejected" | Market closed or invalid | Verify market is active |
