-- Add migration script here
DROP TABLE IF EXISTS token_holders CASCADE;
DROP TABLE IF EXISTS trades CASCADE;
DROP TABLE IF EXISTS tokens CASCADE;

-- Tokens table (matches CreateEvent - now with 12 fields!)
CREATE TABLE tokens (
    mint_address VARCHAR(44) PRIMARY KEY NOT NULL,
    name VARCHAR(100) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    uri TEXT NOT NULL,
    bonding_curve_address VARCHAR(44) NOT NULL,
    creator_wallet VARCHAR(44) NOT NULL,
    
    -- Bonding curve state from CreateEvent
    virtual_token_reserves BIGINT DEFAULT 0,
    virtual_sol_reserves BIGINT DEFAULT 0,
    real_token_reserves BIGINT DEFAULT 0,
    token_total_supply BIGINT DEFAULT 0,
    
    -- Calculated fields
    market_cap_usd DECIMAL(20, 2) DEFAULT 0,
    bonding_curve_progress DECIMAL(5, 2) DEFAULT 0,
    
    -- Status
    complete BOOLEAN DEFAULT FALSE,
    
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT (NOW() AT TIME ZONE 'utc')
);

-- Trades table (ALL 22 fields from TradeEvent!)
CREATE TABLE trades (
    signature VARCHAR(88) PRIMARY KEY NOT NULL,
    token_mint VARCHAR(44) NOT NULL REFERENCES tokens(mint_address) ON DELETE CASCADE,
    
    -- Core trade data
    sol_amount BIGINT NOT NULL,
    token_amount BIGINT NOT NULL,
    is_buy BOOLEAN NOT NULL,
    user_wallet VARCHAR(44) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    
    -- Reserve state at time of trade
    virtual_sol_reserves BIGINT NOT NULL,
    virtual_token_reserves BIGINT NOT NULL,
    real_sol_reserves BIGINT NOT NULL,
    real_token_reserves BIGINT NOT NULL,
    
    -- Fee information (NEW!)
    fee_recipient VARCHAR(44) NOT NULL,
    fee_basis_points BIGINT NOT NULL,
    fee BIGINT NOT NULL,
    
    -- Creator fee (NEW!)
    creator VARCHAR(44) NOT NULL,
    creator_fee_basis_points BIGINT NOT NULL,
    creator_fee BIGINT NOT NULL,
    
    -- Volume tracking (NEW!)
    track_volume BOOLEAN NOT NULL,
    total_unclaimed_tokens BIGINT NOT NULL,
    total_claimed_tokens BIGINT NOT NULL,
    current_sol_volume BIGINT NOT NULL,
    last_update_timestamp TIMESTAMPTZ NOT NULL,
    
    -- Instruction name (NEW!)
    ix_name VARCHAR(20) NOT NULL, -- "buy" | "sell" | "buy_exact_sol_in"
    
    -- Calculated
    price_usd DECIMAL(20, 10)
);

-- Token holders table
CREATE TABLE token_holders (
    id SERIAL PRIMARY KEY,
    token_mint VARCHAR(44) NOT NULL REFERENCES tokens(mint_address) ON DELETE CASCADE,
    user_wallet VARCHAR(44) NOT NULL,
    balance BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE(token_mint, user_wallet)
);

-- Indexes
CREATE INDEX idx_tokens_creator ON tokens(creator_wallet);
CREATE INDEX idx_tokens_created_at ON tokens(created_at DESC);
CREATE INDEX idx_tokens_market_cap ON tokens(market_cap_usd DESC);
CREATE INDEX idx_tokens_complete ON tokens(complete);
CREATE INDEX idx_trades_token_timestamp ON trades(token_mint, timestamp DESC);
CREATE INDEX idx_trades_user ON trades(user_wallet);
CREATE INDEX idx_trades_ix_name ON trades(ix_name); -- NEW: filter by buy/sell
CREATE INDEX idx_trades_creator ON trades(creator); -- NEW: creator trades
CREATE INDEX idx_holders_token ON token_holders(token_mint);
CREATE INDEX idx_holders_balance ON token_holders(balance DESC);