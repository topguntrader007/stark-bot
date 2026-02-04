# Keystore API Server Plan

**URL:** `https://keystore.defirelay.com`

## Overview

A secure blob storage service for encrypted API key backups. The server never sees decrypted data - it just stores and retrieves encrypted strings keyed by wallet address.

The stark-backend handles all encryption/decryption using ECIES with the burner wallet private key. This service stores the encrypted blobs with SIWE (Sign-In With Ethereum) authentication.

**Security Model:** Uses SIWE for authentication - wallet owners must sign a challenge to prove ownership before accessing their data.

## Tech Stack
- **Runtime:** Node.js/Express
- **Database:** PostgreSQL
- **Hosting:** Railway, Fly.io, or similar
- **Dependencies:** ethers.js (for signature verification), siwe (for SIWE message parsing)

---

## Database Schema

```sql
CREATE TABLE backups (
    id SERIAL PRIMARY KEY,
    wallet_id VARCHAR(42) NOT NULL UNIQUE,  -- Ethereum address (0x...)
    encrypted_data TEXT NOT NULL,            -- Hex-encoded ECIES encrypted blob
    key_count INTEGER NOT NULL DEFAULT 0,    -- Informational only
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE sessions (
    id SERIAL PRIMARY KEY,
    wallet_id VARCHAR(42) NOT NULL,
    token VARCHAR(64) NOT NULL UNIQUE,
    nonce VARCHAR(64),
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_backups_wallet_id ON backups(wallet_id);
CREATE INDEX idx_sessions_token ON sessions(token);
CREATE INDEX idx_sessions_wallet_id ON sessions(wallet_id);
```

---

## API Endpoints

### Authentication Flow

#### 1. `POST /api/authorize` - Request Challenge

Start SIWE authentication by requesting a challenge message.

**Request:**
```json
{
  "address": "0x79C62Ff1eE7A0fb038A73fc358DA4306C15CaB6C"
}
```

**Response (200):**
```json
{
  "success": true,
  "message": "keystore.defirelay.com wants you to sign in with your Ethereum account:\n0x79C62Ff1eE7A0fb038A73fc358DA4306C15CaB6C\n\nSign in to Keystore API\n\nURI: https://keystore.defirelay.com\nVersion: 1\nChain ID: 1\nNonce: abc123xyz\nIssued At: 2026-02-01T12:00:00.000Z",
  "nonce": "abc123xyz"
}
```

---

#### 2. `POST /api/authorize/verify` - Verify Signature

Complete authentication by submitting the signed message.

**Request:**
```json
{
  "address": "0x79C62Ff1eE7A0fb038A73fc358DA4306C15CaB6C",
  "signature": "0x1234abcd..."
}
```

**Response (200):**
```json
{
  "success": true,
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "expires_at": "2026-02-01T13:00:00.000Z"
}
```

**Response (401):**
```json
{
  "success": false,
  "error": "Invalid signature"
}
```

---

### Protected Endpoints (Require Bearer Token)

#### 3. `POST /api/store_keys` - Store Encrypted Keys

**Headers:**
```
Authorization: Bearer <token>
```

**Request:**
```json
{
  "encrypted_data": "04a3f2b1c4d5e6f7...",
  "key_count": 5
}
```

**Response (200):**
```json
{
  "success": true,
  "message": "Keys stored successfully",
  "key_count": 5,
  "updated_at": "2026-02-01T12:00:00Z"
}
```

---

#### 4. `POST /api/get_keys` - Retrieve Encrypted Keys

**Headers:**
```
Authorization: Bearer <token>
```

**Response (200):**
```json
{
  "success": true,
  "encrypted_data": "04a3f2b1c4d5e6f7...",
  "key_count": 5,
  "updated_at": "2026-02-01T12:00:00Z"
}
```

**Response (404):**
```json
{
  "success": false,
  "error": "No backup found for this wallet"
}
```

---

#### 5. `DELETE /api/keys` - Delete Backup

**Headers:**
```
Authorization: Bearer <token>
```

**Response (200):**
```json
{
  "success": true,
  "message": "Backup deleted"
}
```

---

### Public Endpoints

#### 6. `GET /api/health` - Health Check

**Response (200):**
```json
{
  "status": "ok"
}
```

---

## Security

1. **SIWE Authentication** - Wallet owners must sign a challenge to prove ownership
2. **Session tokens** - Short-lived tokens (1 hour) reduce signing overhead
3. **Nonce validation** - Prevents replay attacks
4. **Rate limiting** - 10 auth attempts/min, 30 requests/min per IP
5. **Size limit** - Max 1MB for `encrypted_data`
6. **HTTPS only**
7. **CORS** - Allow from your frontend domains
8. **Data encryption** - Server only stores encrypted blobs, cannot read contents

---

## Example Implementation (Node.js)

```javascript
const express = require('express');
const { Pool } = require('pg');
const { ethers } = require('ethers');
const { SiweMessage, generateNonce } = require('siwe');
const cors = require('cors');
const rateLimit = require('express-rate-limit');
const crypto = require('crypto');

const app = express();
const pool = new Pool({ connectionString: process.env.DATABASE_URL });

const SESSION_DURATION_MS = 60 * 60 * 1000; // 1 hour
const DOMAIN = 'keystore.defirelay.com';

app.use(cors({ origin: ['https://stark.defirelay.com', 'http://localhost:5173'] }));
app.use(express.json({ limit: '1mb' }));

// Rate limiting
const authLimiter = rateLimit({ windowMs: 60000, max: 10 });
const apiLimiter = rateLimit({ windowMs: 60000, max: 30 });

// Generate a random token
function generateToken() {
  return crypto.randomBytes(32).toString('hex');
}

// Middleware to validate session token
async function requireAuth(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader?.startsWith('Bearer ')) {
    return res.status(401).json({ success: false, error: 'No token provided' });
  }

  const token = authHeader.slice(7);

  try {
    const result = await pool.query(
      'SELECT wallet_id, expires_at FROM sessions WHERE token = $1',
      [token]
    );

    if (result.rows.length === 0) {
      return res.status(401).json({ success: false, error: 'Invalid token' });
    }

    const session = result.rows[0];
    if (new Date(session.expires_at) < new Date()) {
      await pool.query('DELETE FROM sessions WHERE token = $1', [token]);
      return res.status(401).json({ success: false, error: 'Token expired' });
    }

    req.walletId = session.wallet_id.toLowerCase();
    next();
  } catch (err) {
    console.error('Auth error:', err);
    res.status(500).json({ success: false, error: 'Authentication error' });
  }
}

// Health check
app.get('/api/health', (req, res) => {
  res.json({ status: 'ok' });
});

// Step 1: Request challenge
app.post('/api/authorize', authLimiter, async (req, res) => {
  const { address } = req.body;

  if (!address?.match(/^0x[a-fA-F0-9]{40}$/i)) {
    return res.status(400).json({ success: false, error: 'Invalid address format' });
  }

  const nonce = generateNonce();
  const issuedAt = new Date().toISOString();

  // Create SIWE message
  const siweMessage = new SiweMessage({
    domain: DOMAIN,
    address: address,
    statement: 'Sign in to Keystore API',
    uri: `https://${DOMAIN}`,
    version: '1',
    chainId: 1,
    nonce: nonce,
    issuedAt: issuedAt,
  });

  const message = siweMessage.prepareMessage();

  // Store nonce temporarily (could use Redis for production)
  await pool.query(
    `INSERT INTO sessions (wallet_id, token, nonce, expires_at)
     VALUES (LOWER($1), $2, $3, NOW() + INTERVAL '5 minutes')
     ON CONFLICT (wallet_id) DO UPDATE SET nonce = $3, expires_at = NOW() + INTERVAL '5 minutes'`,
    [address, generateToken(), nonce]
  );

  res.json({ success: true, message, nonce });
});

// Step 2: Verify signature
app.post('/api/authorize/verify', authLimiter, async (req, res) => {
  const { address, signature } = req.body;

  if (!address?.match(/^0x[a-fA-F0-9]{40}$/i)) {
    return res.status(400).json({ success: false, error: 'Invalid address format' });
  }

  try {
    // Get stored nonce
    const nonceResult = await pool.query(
      'SELECT nonce FROM sessions WHERE wallet_id = LOWER($1) AND expires_at > NOW()',
      [address]
    );

    if (nonceResult.rows.length === 0) {
      return res.status(400).json({ success: false, error: 'No pending authorization' });
    }

    const storedNonce = nonceResult.rows[0].nonce;

    // Verify signature
    const recoveredAddress = ethers.verifyMessage(
      `keystore.defirelay.com wants you to sign in with your Ethereum account:\n${address}\n\nSign in to Keystore API\n\nURI: https://keystore.defirelay.com\nVersion: 1\nChain ID: 1\nNonce: ${storedNonce}\nIssued At: `,
      signature
    );

    // Note: In production, parse and verify the full SIWE message properly
    // This is simplified for illustration

    if (recoveredAddress.toLowerCase() !== address.toLowerCase()) {
      return res.status(401).json({ success: false, error: 'Invalid signature' });
    }

    // Create session
    const token = generateToken();
    const expiresAt = new Date(Date.now() + SESSION_DURATION_MS);

    await pool.query(
      `UPDATE sessions SET token = $1, nonce = NULL, expires_at = $2 WHERE wallet_id = LOWER($3)`,
      [token, expiresAt, address]
    );

    res.json({
      success: true,
      token,
      expires_at: expiresAt.toISOString(),
    });
  } catch (err) {
    console.error('Verify error:', err);
    res.status(401).json({ success: false, error: 'Verification failed' });
  }
});

// Store keys (authenticated)
app.post('/api/store_keys', apiLimiter, requireAuth, async (req, res) => {
  const { encrypted_data, key_count } = req.body;

  if (!encrypted_data || !/^[a-fA-F0-9]+$/.test(encrypted_data)) {
    return res.status(400).json({ success: false, error: 'Invalid encrypted_data format' });
  }

  try {
    const result = await pool.query(`
      INSERT INTO backups (wallet_id, encrypted_data, key_count, updated_at)
      VALUES ($1, $2, $3, NOW())
      ON CONFLICT (wallet_id)
      DO UPDATE SET encrypted_data = $2, key_count = $3, updated_at = NOW()
      RETURNING updated_at
    `, [req.walletId, encrypted_data, key_count || 0]);

    res.json({
      success: true,
      message: 'Keys stored successfully',
      key_count: key_count || 0,
      updated_at: result.rows[0].updated_at,
    });
  } catch (err) {
    console.error('Store error:', err);
    res.status(500).json({ success: false, error: 'Database error' });
  }
});

// Get keys (authenticated)
app.post('/api/get_keys', apiLimiter, requireAuth, async (req, res) => {
  try {
    const result = await pool.query(
      'SELECT encrypted_data, key_count, updated_at FROM backups WHERE wallet_id = $1',
      [req.walletId]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ success: false, error: 'No backup found for this wallet' });
    }

    const backup = result.rows[0];
    res.json({
      success: true,
      encrypted_data: backup.encrypted_data,
      key_count: backup.key_count,
      updated_at: backup.updated_at,
    });
  } catch (err) {
    console.error('Get error:', err);
    res.status(500).json({ success: false, error: 'Database error' });
  }
});

// Delete keys (authenticated)
app.delete('/api/keys', apiLimiter, requireAuth, async (req, res) => {
  try {
    const result = await pool.query(
      'DELETE FROM backups WHERE wallet_id = $1 RETURNING id',
      [req.walletId]
    );

    if (result.rowCount === 0) {
      return res.status(404).json({ success: false, error: 'No backup found' });
    }

    res.json({ success: true, message: 'Backup deleted' });
  } catch (err) {
    console.error('Delete error:', err);
    res.status(500).json({ success: false, error: 'Database error' });
  }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => console.log(`Keystore API running on port ${PORT}`));
```

---

## Deployment Checklist

1. Create PostgreSQL database
2. Run schema migration (create both tables)
3. Install dependencies: `npm install express pg ethers siwe cors express-rate-limit`
4. Set environment variables:
   - `DATABASE_URL`
   - `PORT`
5. Deploy to Railway/Fly.io
6. Configure DNS for `keystore.defirelay.com`
7. Verify HTTPS is working
8. Test authentication flow:
   ```bash
   # Health check
   curl https://keystore.defirelay.com/api/health

   # Get challenge (need to sign this message with wallet)
   curl -X POST https://keystore.defirelay.com/api/authorize \
     -H "Content-Type: application/json" \
     -d '{"address":"0x742d35Cc6634C0532925a3b844Bc454e4438f44e"}'
   ```

---

## Client Implementation

The stark-backend already has a `KeystoreClient` in `src/keystore_client.rs` that implements:

1. **Session caching** - Stores tokens and reuses until expiry
2. **Automatic re-authentication** - If token expires, re-authenticates transparently
3. **SIWE signing** - Signs challenge messages with the burner wallet

No changes needed to the stark-backend - it's already compatible with this API design.
