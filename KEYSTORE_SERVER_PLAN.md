# Keystore API Server Plan

**URL:** `https://keystore.defirelay.com`

## Overview

A simple blob storage service for encrypted API key backups. The server never sees decrypted data - it just stores and retrieves encrypted strings keyed by wallet address.

The stark-backend handles all encryption/decryption using ECIES with the burner wallet private key. This service just stores the encrypted blobs.

## Tech Stack
- **Runtime:** Node.js/Express or Rust/Actix-web
- **Database:** PostgreSQL (or SQLite for simplicity)
- **Hosting:** Railway, Fly.io, or similar

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

CREATE INDEX idx_backups_wallet_id ON backups(wallet_id);
```

---

## API Endpoints

### 1. `POST /api/backup` - Store/Update Backup

**Request:**
```json
{
  "wallet_id": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
  "encrypted_data": "04a3f2b1c4d5e6f7...",
  "key_count": 5
}
```

**Response (200):**
```json
{
  "success": true,
  "message": "Backup stored",
  "updated_at": "2026-02-01T12:00:00Z"
}
```

**Response (400):**
```json
{
  "success": false,
  "error": "Invalid wallet_id format"
}
```

**Logic:**
- Validate `wallet_id` matches `/^0x[a-fA-F0-9]{40}$/`
- Validate `encrypted_data` is non-empty hex string
- Upsert: update if exists, insert if new

---

### 2. `GET /api/backup/:wallet_id` - Retrieve Backup

**Response (200):**
```json
{
  "wallet_id": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
  "encrypted_data": "04a3f2b1c4d5e6f7...",
  "key_count": 5,
  "updated_at": "2026-02-01T12:00:00Z"
}
```

**Response (404):**
```json
{
  "error": "No backup found for this wallet"
}
```

---

### 3. `GET /api/health` - Health Check

**Response (200):**
```json
{
  "status": "ok"
}
```

---

## Security

1. **No authentication required** - The ECIES encryption IS the authentication. Only someone with the private key can decrypt the data.
2. **Rate limiting** - 10 writes/min, 30 reads/min per IP
3. **Size limit** - Max 1MB for `encrypted_data`
4. **HTTPS only**
5. **CORS** - Allow from your frontend domains

---

## Example Implementation (Node.js)

```javascript
const express = require('express');
const { Pool } = require('pg');
const cors = require('cors');
const rateLimit = require('express-rate-limit');

const app = express();
const pool = new Pool({ connectionString: process.env.DATABASE_URL });

app.use(cors({ origin: ['https://stark.defirelay.com', 'http://localhost:5173'] }));
app.use(express.json({ limit: '1mb' }));

// Rate limiting
const writeLimiter = rateLimit({ windowMs: 60000, max: 10 });
const readLimiter = rateLimit({ windowMs: 60000, max: 30 });

// Health check
app.get('/api/health', (req, res) => {
  res.json({ status: 'ok' });
});

// Store backup
app.post('/api/backup', writeLimiter, async (req, res) => {
  const { wallet_id, encrypted_data, key_count } = req.body;

  // Validate wallet address format
  if (!wallet_id?.match(/^0x[a-fA-F0-9]{40}$/i)) {
    return res.status(400).json({ success: false, error: 'Invalid wallet_id format' });
  }

  // Validate encrypted data is hex
  if (!encrypted_data || !/^[a-fA-F0-9]+$/.test(encrypted_data)) {
    return res.status(400).json({ success: false, error: 'Invalid encrypted_data format' });
  }

  try {
    const result = await pool.query(`
      INSERT INTO backups (wallet_id, encrypted_data, key_count, updated_at)
      VALUES (LOWER($1), $2, $3, NOW())
      ON CONFLICT (wallet_id)
      DO UPDATE SET encrypted_data = $2, key_count = $3, updated_at = NOW()
      RETURNING updated_at
    `, [wallet_id, encrypted_data, key_count || 0]);

    res.json({
      success: true,
      message: 'Backup stored',
      updated_at: result.rows[0].updated_at
    });
  } catch (err) {
    console.error('Database error:', err);
    res.status(500).json({ success: false, error: 'Database error' });
  }
});

// Retrieve backup
app.get('/api/backup/:wallet_id', readLimiter, async (req, res) => {
  const { wallet_id } = req.params;

  // Validate format
  if (!wallet_id?.match(/^0x[a-fA-F0-9]{40}$/i)) {
    return res.status(400).json({ error: 'Invalid wallet_id format' });
  }

  try {
    const result = await pool.query(
      'SELECT wallet_id, encrypted_data, key_count, updated_at FROM backups WHERE wallet_id = LOWER($1)',
      [wallet_id]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'No backup found for this wallet' });
    }

    res.json(result.rows[0]);
  } catch (err) {
    console.error('Database error:', err);
    res.status(500).json({ error: 'Database error' });
  }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => console.log(`Keystore API running on port ${PORT}`));
```

---

## Deployment Checklist

1. Create PostgreSQL database
2. Run schema migration
3. Set environment variables:
   - `DATABASE_URL`
   - `PORT`
4. Deploy to Railway/Fly.io
5. Configure DNS for `keystore.defirelay.com`
6. Verify HTTPS is working
7. Test with curl:
   ```bash
   # Store
   curl -X POST https://keystore.defirelay.com/api/backup \
     -H "Content-Type: application/json" \
     -d '{"wallet_id":"0x123...","encrypted_data":"abc123","key_count":1}'

   # Retrieve
   curl https://keystore.defirelay.com/api/backup/0x123...
   ```
