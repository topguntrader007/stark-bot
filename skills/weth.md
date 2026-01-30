---
name: weth
description: "Wrap ETH to WETH or unwrap WETH to ETH on Base or Mainnet"
version: 2.0.0
author: starkbot
metadata: {"clawdbot":{"emoji":"🔄"}}
tags: [crypto, defi, weth, wrap, unwrap, base]
---

# WETH Wrap/Unwrap

Convert between ETH and WETH (Wrapped Ether) using presets.

---

## Wrap ETH to WETH

### 1. Get wallet address
```json
{"action": "address", "cache_as": "wallet_address"}
```
**Tool:** `local_burner_wallet`

### 2. Set amount to wrap (in wei)
```json
{"key": "wrap_amount", "value": "1000000000000000"}
```
**Tool:** `register_set`

### 3. Execute wrap
```json
{"preset": "weth_deposit", "network": "base"}
```
**Tool:** `web3_function_call`

---

## Unwrap WETH to ETH

### 1. Get wallet address
```json
{"action": "address", "cache_as": "wallet_address"}
```
**Tool:** `local_burner_wallet`

### 2. Set amount to unwrap (in wei)
```json
{"key": "unwrap_amount", "value": "1000000000000000"}
```
**Tool:** `register_set`

### 3. Execute unwrap
```json
{"preset": "weth_withdraw", "network": "base"}
```
**Tool:** `web3_function_call`

---

## Check WETH Balance

### 1. Get wallet address first (sets wallet_address register)
```json
{"action": "address", "cache_as": "wallet_address"}
```
**Tool:** `local_burner_wallet`

### 2. Check balance
```json
{"preset": "weth_balance", "network": "base", "call_only": true}
```
**Tool:** `web3_function_call`

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
| `weth_balance` | Check WETH balance | `wallet_address` |

---

## Why Use WETH?

- Many DeFi protocols require ERC20 tokens, not native ETH
- WETH is a 1:1 wrapped version of ETH as an ERC20
- Wrapping/unwrapping is instant and costs only gas
- Some DEX swaps automatically wrap ETH, but direct WETH control is sometimes needed
