---
name: broadcast_transactions
description: "Manage and broadcast queued blockchain transactions"
version: 1.0.0
author: starkbot
metadata: {"clawdbot":{"emoji":"📡"}}
tags: [crypto, transaction, queue, broadcast, base, ethereum]
requires_tools: [broadcast_web3_tx, list_queued_web3_tx]
---

# Transaction Queue System

This skill explains how to work with the transaction queue system.

## Overview

Transactions from `web3_tx` are **queued** rather than broadcast immediately. This creates a safety layer where transactions can be reviewed before sending.

### Flow

1. `web3_tx` signs transaction and queues it (returns UUID)
2. `list_queued_web3_tx` shows queued transactions
3. `broadcast_web3_tx` broadcasts by UUID

---

## List Queued Transactions

View all queued transactions:
```tool:list_queued_web3_tx
```

View only pending transactions:
```tool:list_queued_web3_tx
status: pending
```

View a specific transaction:
```tool:list_queued_web3_tx
uuid: <UUID>
```

Filter options:
- `pending` - Signed but not broadcast
- `broadcasting` - Currently being broadcast
- `broadcast` - Sent to network, awaiting confirmation
- `confirmed` - Confirmed on-chain
- `failed` - Broadcast or confirmation failed
- `expired` - Transaction timed out

---

## Broadcast a Transaction

To broadcast a pending transaction:
```tool:broadcast_web3_tx
uuid: <UUID_FROM_WEB3_TX>
```

This will:
1. Retrieve the signed transaction from the queue
2. Broadcast it to the network
3. Wait for confirmation
4. Return the transaction hash and explorer URL

---

## Transaction Statuses

| Status | Description |
|--------|-------------|
| **pending** | Signed and waiting to be broadcast |
| **broadcasting** | Currently being sent to the network |
| **broadcast** | Sent to network, awaiting block confirmation |
| **confirmed** | Successfully included in a block |
| **failed** | Transaction failed (see error field) |
| **expired** | Transaction timed out before broadcast |

---

## Complete Workflow Example

### 1. Queue a swap transaction

After preparing the swap quote:
```tool:web3_tx
from_register: swap_quote
max_fee_per_gas: "<GAS_PRICE>"
network: base
```

Response includes:
```
TRANSACTION QUEUED (not yet broadcast)

UUID: abc12345-...
Network: base
To: 0x...
Value: 100000000000000 wei (0.0001 ETH)
...
```

### 2. Verify the queued transaction

```tool:list_queued_web3_tx
status: pending
```

### 3. Broadcast when ready

```tool:broadcast_web3_tx
uuid: abc12345-...
```

Response includes:
```
TRANSACTION CONFIRMED

Hash: 0x...
Explorer: https://basescan.org/tx/0x...
```

---

## Error Handling

| Error | Cause | Solution |
|-------|-------|----------|
| "UUID not found" | Invalid or expired UUID | Use `list_queued_web3_tx` to find valid UUIDs |
| "Already broadcast" | Transaction was already sent | Check explorer with the tx_hash |
| "Already being broadcast" | Concurrent broadcast attempt | Wait for the first broadcast to complete |
| "Transaction failed" | RPC or network error | Check the error message, may need new transaction |
| "Nonce too low" | Transaction with this nonce already mined | Create a new transaction with updated nonce |

---

## Best Practices

1. **Always verify before broadcasting** - Use `list_queued_web3_tx` to review transaction details
2. **One broadcast at a time** - Don't broadcast multiple transactions with the same nonce
3. **Check pending count** - If pending count is high, review which transactions to broadcast
4. **Handle failures gracefully** - Failed transactions may need to be re-created with new parameters
