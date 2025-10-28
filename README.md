#  Pump.fun Indexer

A high-performance, production-ready indexer for [pump.fun](https://pump.fun) - the Solana token launchpad. Indexes all token creations, trades, and graduations in real-time with historical backfill support.

---

##  **What Does This Indexer Do?**

This indexer:
-  **Listens** to all pump.fun transactions in real-time via Helius WebSocket
-  **Parses** token creation, trade, and completion events
-  **Calculates** live market caps, bonding curve progress, and prices
-  **Stores** data in Postgres for historical analysis
-  **Streams** real-time updates via WebSocket
-  **Exposes** REST API for querying tokens, trades, and statistics
-  **Backfills** historical data from any point in time

---

##  **Architecture Overview**

![Pump.fun Indexer Architecture](./docs/architecture.png)

### **System Components**

#### **1. Data Ingestion Layer**
- **Helius Listener**: Subscribes to pump.fun program via WebSocket for real-time transactions
- **Backfill Worker**: Fetches historical transactions via Helius RPC
- **Event Parser**: 
  - Borsh deserialization of on-chain data
  - Base64/Base58 decoding
  - Discriminator matching for event types (CREATE/TRADE/COMPLETE)

#### **2. Event Processing Layer**
- **Event Processor**: Routes events to appropriate handlers
  - `CREATE` → Initialize token state + save to database
  - `TRADE` → Update reserves + publish to Redis
  - `COMPLETE` → Mark token as graduated
- **State Manager**: In-memory `Arc<RwLock<HashMap>>` for live token state
  - Updates reserves in real-time
  - Calculates token prices
  - Computes market cap using SOL price

#### **3. Storage Layer**
- **Postgres Pool**: Persistent storage for:
  - Tokens (mint, name, symbol, creator, reserves)
  - Trades (signature, user, amounts, timestamp)
  - Transactions (signature, slot, success/error)
- **Redis Pub/Sub**: Real-time event broadcasting:
  - `pump:trades` - All trades channel
  - `pump:tokens:new` - New token launches
  - `pump:completions` - Token graduations

#### **4. Background Services**
- **SOL Price Updater**: Fetches SOL/USD price every 15 seconds
  - Primary: Pyth Network
  - Fallback: CoinGecko API
- **State Backup**: Persists in-memory state to Postgres every 60 seconds
  - Prevents data loss on restart
  - Updates market caps in database

#### **5. API Layer**
- **REST API** (Axum framework):
  - `GET /api/tokens` - List all tokens with pagination
  - `GET /api/tokens/{mint}` - Token details
  - `GET /api/tokens/{mint}/trades` - Trade history
  - `GET /api/creators/{wallet}` - Creator's tokens
  - `GET /api/stats` - Global statistics
- **WebSocket API**:
  - `WS /ws/trades` - Stream all trades
  - `WS /ws/trades/{mint}` - Token-specific trade stream

#### **6. Clients**
- Web frontends (React/Vue dashboards)
- Trading bots (automated strategies)
- Analytics platforms (data visualization)

### **Data Flow Example**

```
1. New transaction on Solana
   ↓
2. Helius WebSocket pushes to indexer
   ↓
3. Parser extracts TRADE event
   ↓
4. State Manager updates token reserves
   ↓
5. Calculator computes new price/market cap
   ↓
6. Storage Router saves to Postgres + publishes to Redis
   ↓
7. WebSocket clients receive real-time update
   ↓
8. Frontend displays live trade
```

---

##  **Features**

### **Data Collection**
- [x] Real-time event streaming (Helius WebSocket)
- [x] Historical backfill (Helius RPC)
- [x] Automatic retry logic with exponential backoff
- [x] Duplicate event detection
- [x] Failed transaction handling

### **Event Types**
- [x] **CREATE**: New token launches
  - Extracts: mint, name, symbol, creator, bonding curve
  - Initializes: virtual/real reserves, total supply
- [x] **TRADE**: Buy/sell transactions
  - Captures: trader wallet, is_buy flag, amounts
  - Updates: reserves, fees, creator fees
- [x] **COMPLETE**: Token graduation to Raydium
  - Marks token as graduated
  - Records final reserves

### **Calculations**
- [x] Live SOL/USD price (Pyth + CoinGecko fallback)
- [x] Token price in SOL (virtual reserves ratio)
- [x] Market capitalization (SOL + USD)
- [x] Bonding curve progress (0-100%)
- [x] Price impact estimation
- [x] 24h volume tracking

### **APIs**
- [x] **REST API** (Axum)
  - Token queries with filtering/sorting
  - Trade history with pagination
  - Creator analytics
  - Global statistics
  - Health checks
- [x] **WebSocket API**
  - Real-time trade streams
  - Token-specific subscriptions
  - Auto-reconnect on disconnect

### **Background Services**
- [x] SOL price updater (15s interval)
- [x] State backup to Postgres (60s interval)
- [x] Redis pub/sub for real-time updates
- [x] Automatic state recovery on restart

---

##  **Tech Stack**

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Language** | Rust 1.70+ | High performance, memory safety |
| **Async Runtime** | Tokio | Concurrent task execution |
| **Database** | PostgreSQL 15+ | Persistent storage |
| **Cache/Pub-Sub** | Redis 7+ | Real-time event distribution |
| **RPC Provider** | Helius | WebSocket + RPC access |
| **Price Oracle** | Pyth + CoinGecko | SOL/USD price feeds |
| **Web Framework** | Axum | REST + WebSocket API |
| **ORM** | SQLx | Compile-time SQL verification |
| **Serialization** | Borsh | Solana program data parsing |

---

## 🚀 **Quick Start**

### **Prerequisites**

- **Rust 1.70+**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **PostgreSQL 15+**: For persistent storage
- **Redis 7+**: For pub/sub messaging
- **Helius API Key**: Free tier available at [helius.dev](https://helius.dev)

### **1. Clone & Setup**

```bash
git clone https://github.com/yourusername/pumpfun-indexer.git
cd pumpfun-indexer

# Install dependencies
cargo build --release
```

### **2. Environment Setup**

Create `.env` file in project root:

```env
# Database
DATABASE_URL=postgresql://user:password@localhost:5432/pump_indexer

# Helius (REQUIRED)
HELIUS_API_KEY=your_helius_key_here

# Redis
REDIS_URL=redis://localhost:6379

# API Server
API_PORT=8080

# Optional: CoinGecko Pro API (higher rate limits)
COINGECKO_API_KEY=your_coingecko_key
```

### **3. Database Migration**

```bash
# Install sqlx-cli (one-time setup)
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run
```

### **4. Start Services**

#### **Option A: Docker (Recommended)**

```bash
# Start Postgres + Redis
docker-compose up -d

# Start indexer
cargo run --release
```

#### **Option B: Manual Setup**

```bash
# Terminal 1: Start Postgres
docker run -d --name postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=pump_indexer \
  -p 5432:5432 \
  postgres:15

# Terminal 2: Start Redis
docker run -d --name redis -p 6379:6379 redis:7

# Terminal 3: Start Indexer
cargo run --release
```

### **Expected Output**

```
🚀 Starting Pump.fun Indexer...
✅ Configuration loaded
✅ Database connected (20 connections in pool)
✅ Redis connected
🔗 Helius WebSocket connected
💰 SOL Price: $152.34 USD
🌐 Starting API server on 0.0.0.0:8080
✨ Indexer is running!
   REST API: http://localhost:8080/api
   WebSocket: ws://localhost:8080/ws/trades
   Health: http://localhost:8080/health
```

---

##  **Historical Backfill**

The indexer supports two-phase backfilling to efficiently populate historical data.

### **Why Two-Phase?**

1. **Phase 1 (Tokens)**: Collects all CREATE events first
2. **Phase 2 (Trades)**: Processes trades that reference existing tokens
3. **Prevents**: Foreign key errors from trades without token records

### **Phase 1: Backfill Tokens**

```bash
# Test run (100 transactions)
cargo run --release --bin backfill -- --tokens-only --max-txs 100

# Medium run (10,000 transactions ~ 100-200 tokens)
cargo run --release --bin backfill -- --tokens-only --max-txs 10000

# Full history (all tokens since pump.fun launch)
cargo run --release --bin backfill -- --tokens-only
```

**Expected Output:**
```
🚀 Starting Pump.fun Backfill Tool
   📍 MODE: PHASE 1 - TOKENS ONLY
   Will collect: Token creations
   Will skip: Trades, Completions

📡 Batch #1: Fetching up to 1000 signatures...
📥 Processing batch of 1000 signatures...
   ✨ 50 tokens created so far
   ✨ 100 tokens created so far

🎉 Backfill Complete!
   ├─ Tokens created: 127
   ├─ Trades: 0 (skipped in tokens-only mode)
   └─ Completions: 0
   Total time: 2m 34s
```

### **Phase 2: Backfill Trades**

```bash
# Test run
cargo run --release --bin backfill -- --trades-only --max-txs 100

# Medium run
cargo run --release --bin backfill -- --trades-only --max-txs 50000

# Full history
cargo run --release --bin backfill -- --trades-only
```

**Expected Output:**
```
🚀 Starting Pump.fun Backfill Tool
   📍 MODE: PHASE 2 - TRADES ONLY
   Will collect: Trades, Completions
   Will skip: Token creations

📡 Batch #1: Fetching up to 1000 signatures...
   💰 1000 trades saved so far
   💰 2000 trades saved so far

🎉 Backfill Complete!
   ├─ Tokens created: 0 (skipped in trades-only mode)
   ├─ Trades: 2,847
   └─ Completions: 3
   Total time: 8m 12s
```

### **Advanced Options**

```bash
# Start from specific signature (useful for resuming)
cargo run --release --bin backfill -- \
  --before 5Jx7y8mK... \
  --tokens-only \
  --max-txs 50000

# Adjust batch size (default: 1000)
cargo run --release --bin backfill -- \
  --batch-size 500 \
  --tokens-only

# Increase concurrency (default: 10)
cargo run --release --bin backfill -- \
  --concurrency 20 \
  --trades-only

# Run in background
nohup cargo run --release --bin backfill -- --tokens-only > backfill.log 2>&1 &
tail -f backfill.log
```

---

##  **API Usage**

### **REST API Examples**

#### **Get All Tokens**
```bash
# Default (50 tokens, sorted by created_at DESC)
curl http://localhost:8080/api/tokens

# With pagination and sorting
curl "http://localhost:8080/api/tokens?limit=100&offset=50&sort=market_cap"

# Filter by completion status
curl "http://localhost:8080/api/tokens?completed=true"
```

**Response:**
```json
{
  "tokens": [
    {
      "mint_address": "4xLkZHGr...",
      "name": "Doge Coin",
      "symbol": "DOGE",
      "creator_wallet": "CebN2...",
      "market_cap_usd": 125340.50,
      "bonding_curve_progress": 67.8,
      "complete": false,
      "created_at": "2025-10-28T10:30:00Z"
    }
  ],
  "total": 1234,
  "limit": 50,
  "offset": 0
}
```

#### **Get Token Details**
```bash
curl http://localhost:8080/api/tokens/4xLkZHGr...
```

**Response:**
```json
{
  "mint_address": "4xLkZHGr...",
  "name": "Doge Coin",
  "symbol": "DOGE",
  "uri": "https://arweave.net/...",
  "creator_wallet": "CebN2...",
  "bonding_curve_address": "BcA3x...",
  "virtual_sol_reserves": 30000000000,
  "virtual_token_reserves": 1073000000000000,
  "price_sol": 0.000028,
  "market_cap_sol": 28.5,
  "market_cap_usd": 4342.10,
  "bonding_curve_progress": 67.8,
  "complete": false,
  "created_at": "2025-10-28T10:30:00Z",
  "updated_at": "2025-10-28T14:22:15Z"
}
```

#### **Get Token Trades**
```bash
# Recent trades
curl "http://localhost:8080/api/tokens/4xLkZHGr.../trades?limit=100"

# Filter by trade type
curl "http://localhost:8080/api/tokens/4xLkZHGr.../trades?is_buy=true"
```

**Response:**
```json
{
  "trades": [
    {
      "signature": "5Jx7y...",
      "user_wallet": "FyQr3...",
      "is_buy": true,
      "token_amount": 1000000000,
      "sol_amount": 50000000,
      "price_sol": 0.00005,
      "timestamp": "2025-10-28T14:22:10Z"
    }
  ],
  "total": 847,
  "limit": 100,
  "offset": 0
}
```

#### **Get Creator's Tokens**
```bash
curl http://localhost:8080/api/creators/CebN2...
```

**Response:**
```json
{
  "creator_wallet": "CebN2...",
  "tokens": [
    {
      "mint_address": "4xLkZHGr...",
      "name": "Doge Coin",
      "symbol": "DOGE",
      "market_cap_usd": 4342.10,
      "created_at": "2025-10-28T10:30:00Z"
    }
  ],
  "total_tokens": 5,
  "total_volume_usd": 125340.50
}
```

#### **Get Global Stats**
```bash
curl http://localhost:8080/api/stats
```

**Response:**
```json
{
  "total_tokens": 1234,
  "graduated_tokens": 23,
  "total_trades": 45678,
  "total_volume_sol": 12345.67,
  "total_volume_usd": 1883456.78,
  "sol_price_usd": 152.34,
  "last_updated": "2025-10-28T14:22:15Z"
}
```

---

### **WebSocket Examples**

#### **JavaScript (Browser)**
```javascript
// Stream all trades
const ws = new WebSocket('ws://localhost:8080/ws/trades');

ws.onopen = () => {
  console.log('✅ Connected to trade stream');
};

ws.onmessage = (event) => {
  const trade = JSON.parse(event.data);
  console.log(`${trade.is_buy ? 'BUY' : 'SELL'} ${trade.token_amount} tokens for ${trade.sol_amount} SOL`);
};

ws.onerror = (error) => {
  console.error('❌ WebSocket error:', error);
};

ws.onclose = () => {
  console.log('🔌 Connection closed');
};
```

#### **JavaScript (Token-Specific)**
```javascript
// Stream trades for specific token
const tokenMint = '4xLkZHGr...';
const ws = new WebSocket(`ws://localhost:8080/ws/trades/${tokenMint}`);

ws.onmessage = (event) => {
  const trade = JSON.parse(event.data);
  updateChart(trade); // Update your trading chart
};
```

#### **Python**
```python
import websocket
import json

def on_message(ws, message):
    trade = json.loads(message)
    print(f"{'BUY' if trade['is_buy'] else 'SELL'} "
          f"{trade['token_amount']} tokens for {trade['sol_amount']} SOL")

def on_error(ws, error):
    print(f"Error: {error}")

def on_close(ws, close_status_code, close_msg):
    print("Connection closed")

def on_open(ws):
    print("✅ Connected to trade stream")

# Connect to stream
ws = websocket.WebSocketApp(
    "ws://localhost:8080/ws/trades",
    on_open=on_open,
    on_message=on_message,
    on_error=on_error,
    on_close=on_close
)

ws.run_forever()
```

#### **Node.js**
```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:8080/ws/trades');

ws.on('open', () => {
  console.log('✅ Connected');
});

ws.on('message', (data) => {
  const trade = JSON.parse(data);
  console.log('New trade:', trade);
});

ws.on('error', (error) => {
  console.error('Error:', error);
});
```

---

##  **Monitoring**

### **Check Database Status**
```bash
# Run database check utility
cargo run --bin check_db
```

**Output:**
```
 Database Statistics
════════════════════════════════════
 Tokens: 1,234
 Trades: 45,678
 Transactions: 46,912
 Graduated Tokens: 23
════════════════════════════════════
```

### **View Application Logs**

```bash
# Info level (default - recommended for production)
RUST_LOG=info cargo run --release

# Debug level (detailed logging)
RUST_LOG=debug cargo run --release

# Trace level (very verbose - debugging only)
RUST_LOG=trace cargo run --release

# Module-specific logging
RUST_LOG=pumpfun_indexer::helius=debug,pumpfun_indexer::processor=info cargo run
```

### **Health Check Endpoint**
```bash
# Check if indexer is running
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "healthy",
  "database": "connected",
  "redis": "connected",
  "helius_websocket": "connected",
  "sol_price_usd": 152.34,
  "tokens_in_state": 1234,
  "uptime_seconds": 3600
}
```

### **Production Monitoring**

For production deployments, consider:

1. **Prometheus Metrics** (roadmap feature)
   ```bash
   curl http://localhost:9090/metrics
   ```

2. **Grafana Dashboard** (roadmap feature)
   - Transaction throughput
   - WebSocket connections
   - Database query performance
   - Memory usage trends

3. **Log Aggregation**
   - Use `RUST_LOG=json` for structured logging
   - Forward to ELK stack or Datadog

---

##  **Database Schema**

### **Tokens Table**
```sql
CREATE TABLE tokens (
    mint_address TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    uri TEXT NOT NULL,
    creator_wallet TEXT NOT NULL,
    bonding_curve_address TEXT NOT NULL,
    
    -- Reserve tracking
    virtual_sol_reserves BIGINT NOT NULL,
    virtual_token_reserves BIGINT NOT NULL,
    real_token_reserves BIGINT NOT NULL,
    token_total_supply BIGINT NOT NULL,
    
    -- Calculated fields (updated by background task)
    market_cap_usd DOUBLE PRECISION,
    bonding_curve_progress DOUBLE PRECISION,
    
    -- Status
    complete BOOLEAN DEFAULT FALSE,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_tokens_creator ON tokens(creator_wallet);
CREATE INDEX idx_tokens_created_at ON tokens(created_at DESC);
CREATE INDEX idx_tokens_complete ON tokens(complete);
CREATE INDEX idx_tokens_market_cap ON tokens(market_cap_usd DESC NULLS LAST);
```

### **Trades Table**
```sql
CREATE TABLE trades (
    signature TEXT PRIMARY KEY,
    token_mint TEXT NOT NULL REFERENCES tokens(mint_address),
    user_wallet TEXT NOT NULL,
    
    -- Trade details
    is_buy BOOLEAN NOT NULL,
    sol_amount BIGINT NOT NULL,
    token_amount BIGINT NOT NULL,
    
    -- State after trade
    virtual_sol_reserves BIGINT NOT NULL,
    virtual_token_reserves BIGINT NOT NULL,
    real_sol_reserves BIGINT NOT NULL,
    real_token_reserves BIGINT NOT NULL,
    
    -- Fees
    fee_recipient TEXT NOT NULL,
    fee_basis_points BIGINT NOT NULL,
    fee BIGINT NOT NULL,
    creator TEXT NOT NULL,
    creator_fee_basis_points BIGINT NOT NULL,
    creator_fee BIGINT NOT NULL,
    
    -- Metadata
    track_volume BOOLEAN NOT NULL,
    total_unclaimed_tokens BIGINT NOT NULL,
    total_claimed_tokens BIGINT NOT NULL,
    current_sol_volume BIGINT NOT NULL,
    last_update_timestamp TIMESTAMPTZ NOT NULL,
    ix_name TEXT NOT NULL,
    
    timestamp TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_trades_token ON trades(token_mint);
CREATE INDEX idx_trades_user ON trades(user_wallet);
CREATE INDEX idx_trades_timestamp ON trades(timestamp DESC);
CREATE INDEX idx_trades_is_buy ON trades(is_buy);
```

### **Transactions Table**
```sql
CREATE TABLE transactions (
    signature TEXT PRIMARY KEY,
    slot BIGINT NOT NULL,
    block_time TIMESTAMPTZ NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    signer TEXT NOT NULL,
    accounts_involved TEXT[] NOT NULL,
    compute_units_consumed BIGINT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_transactions_slot ON transactions(slot DESC);
CREATE INDEX idx_transactions_signer ON transactions(signer);
CREATE INDEX idx_transactions_success ON transactions(success);
```

---

##  **Performance Benchmarks**

| Metric | Value | Notes |
|--------|-------|-------|
| **Live Throughput** | 50-100 TXs/sec | Limited by Helius WebSocket |
| **Backfill Speed** | 3-5 TXs/sec | Limited by Helius RPC rate limits |
| **API Latency (GET /tokens)** | <50ms | Data from in-memory state |
| **API Latency (GET /trades)** | <100ms | Postgres query with index |
| **WebSocket Latency** | <100ms | Redis pub/sub + network |
| **Memory Usage (Base)** | ~50MB | Minimal state |
| **Memory Usage (10K tokens)** | ~200MB | Full in-memory state |
| **Database Size (1M trades)** | ~500MB | Postgres compressed |

### **Optimization Tips**

1. **Use connection pooling**: Already configured (20 connections)
2. **Enable database indexes**: Run `sqlx migrate run`
3. **Use `--release` build**: 10x faster than debug
4. **Increase Helius plan**: Pro plans = higher rate limits
5. **Use Redis for state**: Reduces memory usage (roadmap)

---

##  **Troubleshooting**

### **Helius WebSocket Disconnects**

**Symptoms:**
```
⚠️  WebSocket connection lost
🔄 Reconnecting in 5 seconds...
```

**Solutions:**
- ✅ Auto-reconnect is built-in (exponential backoff)
- Check Helius dashboard for rate limit usage
- Upgrade Helius plan if hitting limits
- Verify `HELIUS_API_KEY` is correct

### **Missing Tokens in Trades**

**Symptoms:**
```
❌ Foreign key violation: token not found
```

**Solutions:**
```bash
# 1. Clean database
psql $DATABASE_URL -c "DELETE FROM trades; DELETE FROM tokens;"

# 2. Run Phase 1 first
cargo run --release --bin backfill -- --tokens-only --max-txs 50000

# 3. Then run Phase 2
cargo run --release --bin backfill -- --trades-only --max-txs 50000
```

### **Redis Connection Failed**

**Symptoms:**
```
❌ Failed to connect to Redis: Connection refused
```

**Solutions:**
```bash
# Check if Redis is running
docker ps | grep redis

# If not running, start Redis
docker run -d --name redis -p 6379:6379 redis:7

# Test connection
redis-cli ping
# Should return: PONG

# Verify REDIS_URL in .env
echo $REDIS_URL
```

### **High Memory Usage**

**Symptoms:**
```
System memory: 2GB+ used by indexer
```

**Solutions:**
```rust
// Limit tokens in state (modify src/processor/state.rs)
const MAX_TOKENS_IN_STATE: usize = 5000;

// Or use Redis for state (roadmap feature)
```

### **Slow Backfill**

**Solutions:**
```bash
# Increase concurrency
cargo run --release --bin backfill -- --concurrency 20 --tokens-only

# Reduce batch size to avoid timeouts
cargo run --release --bin backfill -- --batch-size 500 --tokens-only

# Check Helius rate limits
curl "https://mainnet.helius-rpc.com/?api-key=$HELIUS_API_KEY" \
  -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

### **Database Migration Errors**

**Symptoms:**
```
❌ Migration failed: column already exists
```

**Solutions:**
```bash
# Reset migrations (WARNING: deletes all data)
sqlx migrate revert --all
sqlx migrate run

# Or manually fix in psql
psql $DATABASE_URL
DROP TABLE IF EXISTS trades CASCADE;
DROP TABLE IF EXISTS tokens CASCADE;
DROP TABLE IF EXISTS transactions CASCADE;
\q
sqlx migrate run
```

---

## 📄 **License**

MIT License

Copyright (c) 2025 [Your Name]

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

##  **Contributing**

I welcome any type of contributions! Here's how to get started:

### **Development Setup**

```bash
# 1. Fork the repository
# 2. Clone your fork
git clone https://github.com/yourusername/pumpfun-indexer.git
cd pumpfun-indexer

# 3. Create a feature branch
git checkout -b feature/amazing-feature

# 4. Make your changes
# 5. Run tests
cargo test

# 6. Check formatting
cargo fmt --check

# 7. Run linter
cargo clippy -- -D warnings

# 8. Commit changes
git commit -m 'Add amazing feature'

# 9. Push to your fork
git push origin feature/amazing-feature

# 10. Open Pull Request
```



---



## 📚 **Resources**

### **External Resources**
- [pump.fun Contracts](https://github.com/pump-fun)
- [Helius Documentation](https://docs.helius.dev)
- [Solana Documentation](https://docs.solana.com)
- [Borsh Specification](https://borsh.io)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [SQLx Documentation](https://github.com/launchbadge/sqlx)

### **Related Projects**
- [Solana Program Library](https://github.com/solana-labs/solana-program-library)
- [Metaplex](https://github.com/metaplex-foundation/metaplex)
- [Serum DEX](https://github.com/project-serum/serum-dex)

---

