---
name: weth
description: "Wrap ETH to WETH or unwrap WETH to ETH on Base or Mainnet"
version: 2.2.0
author: starkbot
metadata: {"clawdbot":{"emoji":"🔄"}}
tags: [crypto, defi, finance, weth, wrap, unwrap, base]
requires_tools: [web, register_set]
---

# WETH Wrap/Unwrap

Convert between ETH and WETH (Wrapped Ether) using presets.

**Note:** `wallet_address` is an intrinsic register - always available automatically. No need to fetch it.

---

## Wrap ETH to WETH

### 1. Set amount to wrap (in wei)
```tool:register_set
key: wrap_amount
value: "1000000000000000"
```

### 2. Execute wrap
```tool:web3_function_call
preset: weth_deposit
network: base
```

---

## Unwrap WETH to ETH

### 1. Set amount to unwrap (in wei)
```tool:register_set
key: unwrap_amount
value: "1000000000000000"
```

### 2. Execute unwrap
```tool:web3_function_call
preset: weth_withdraw
network: base
```

---

## Check WETH Balance

```tool:web3_function_call
preset: weth_balance
network: base
call_only: true
```

The `wallet_address` register is intrinsic - no need to set it first.

---

## Amount Reference (Wei)

| ETH Amount | Wei Value |
|------------|-----------|
| 0.0001 ETH | `100000000000000` |
| 0.001 ETH | `1000000000000000` |
| 0.01 ETH | `10000000000000000` |
| 0.1 ETH | `100000000000000000` |
| 1 ETH | `1000000000000000000` |

---

## Available Presets

| Preset | Description | Required Registers |
|--------|-------------|-------------------|
| `weth_deposit` | Wrap ETH to WETH | `wrap_amount` |
| `weth_withdraw` | Unwrap WETH to ETH | `unwrap_amount` |
| `weth_balance` | Check WETH balance | `wallet_address` (intrinsic) |

---

## Why Use WETH?

- Many DeFi protocols require ERC20 tokens, not native ETH
- WETH is a 1:1 wrapped version of ETH as an ERC20
- Wrapping/unwrapping is instant and costs only gas
- Some DEX swaps automatically wrap ETH, but direct WETH control is sometimes needed

---

## Transaction Queue Note

When using `web3_function_call` with presets like `weth_deposit` or `weth_withdraw`, the transaction is executed directly (not queued).

If you need to review transactions before broadcast, use the register pattern with `web3_tx` instead:

```tool:register_set
key: weth_tx
json_value:
  to: "0x4200000000000000000000000000000000000006"
  value: "1000000000000000"
  data: "0xd0e30db0"
  gas: "50000"
```

```tool:web3_tx
from_register: weth_tx
max_fee_per_gas: "<GAS_PRICE>"
network: base
```

Then verify and broadcast:
```tool:list_queued_web3_tx
status: pending
```

```tool:broadcast_web3_tx
uuid: <UUID>
```
