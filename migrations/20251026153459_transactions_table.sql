-- Drop existing tables and recreate with all columns
DROP TABLE IF EXISTS indexer_stats CASCADE;
DROP TABLE IF EXISTS transactions CASCADE;
DROP TABLE IF EXISTS token_holders CASCADE;
DROP TABLE IF EXISTS trades CASCADE;
DROP TABLE IF EXISTS tokens CASCADE;

-- Tokens table 
CREATE TABLE tokens (
    mint_address VARCHAR(44) PRIMARY KEY NOT NULL,
    name VARCHAR(100) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    uri TEXT NOT NULL,
    bonding_curve_address VARCHAR(44) NOT NULL,
    creator_wallet VARCHAR(44) NOT NULL,
    
    virtual_token_reserves BIGINT DEFAULT 0,
    virtual_sol_reserves BIGINT DEFAULT 0,
    real_token_reserves BIGINT DEFAULT 0,
    token_total_supply BIGINT DEFAULT 0,
    
    market_cap_usd DECIMAL(20, 2) DEFAULT 0,
    bonding_curve_progress DECIMAL(5, 2) DEFAULT 0,

    complete BOOLEAN DEFAULT FALSE,
    
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT (NOW() AT TIME ZONE 'utc')
);

-- Trades table
CREATE TABLE trades (
    signature VARCHAR(88) PRIMARY KEY NOT NULL,
    token_mint VARCHAR(44) NOT NULL REFERENCES tokens(mint_address) ON DELETE CASCADE,
    
    sol_amount BIGINT NOT NULL,
    token_amount BIGINT NOT NULL,
    is_buy BOOLEAN NOT NULL,
    user_wallet VARCHAR(44) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,

    virtual_sol_reserves BIGINT NOT NULL,
    virtual_token_reserves BIGINT NOT NULL,
    real_sol_reserves BIGINT NOT NULL,
    real_token_reserves BIGINT NOT NULL,

    fee_recipient VARCHAR(44) NOT NULL,
    fee_basis_points BIGINT NOT NULL,
    fee BIGINT NOT NULL,

    creator VARCHAR(44) NOT NULL,
    creator_fee_basis_points BIGINT NOT NULL,
    creator_fee BIGINT NOT NULL,

    track_volume BOOLEAN NOT NULL,
    total_unclaimed_tokens BIGINT NOT NULL,
    total_claimed_tokens BIGINT NOT NULL,
    current_sol_volume BIGINT NOT NULL,
    last_update_timestamp TIMESTAMPTZ NOT NULL,

    ix_name VARCHAR(20) NOT NULL,
    
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

-- All pump.fun program interactions (🔥 WITH ALL COLUMNS)
CREATE TABLE transactions (
    signature VARCHAR(88) PRIMARY KEY,
    slot BIGINT NOT NULL,
    block_time TIMESTAMPTZ NOT NULL,
    fee BIGINT NOT NULL,
    success BOOLEAN NOT NULL,
    signer VARCHAR(44) NOT NULL,
    instruction_count INTEGER NOT NULL,
    log_messages_count INTEGER NOT NULL,
    has_program_data BOOLEAN DEFAULT false,
    accounts_involved TEXT[],               -- 🔥 NEW
    pre_balances BIGINT[],                  -- 🔥 NEW
    post_balances BIGINT[],                 -- 🔥 NEW
    compute_units_consumed BIGINT,          -- 🔥 NEW
    error_message TEXT,                      -- 🔥 NEW
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Statistics table
CREATE TABLE indexer_stats (
    id INTEGER PRIMARY KEY DEFAULT 1,
    total_transactions BIGINT DEFAULT 0,
    total_tokens_created BIGINT DEFAULT 0,
    total_trades BIGINT DEFAULT 0,
    total_volume_sol NUMERIC(20, 9) DEFAULT 0,
    last_processed_slot BIGINT DEFAULT 0,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT single_row CHECK (id = 1)
);

-- Indexes for tokens
CREATE INDEX idx_tokens_creator ON tokens(creator_wallet);
CREATE INDEX idx_tokens_created_at ON tokens(created_at DESC);
CREATE INDEX idx_tokens_market_cap ON tokens(market_cap_usd DESC);
CREATE INDEX idx_tokens_complete ON tokens(complete);

-- Indexes for trades
CREATE INDEX idx_trades_token_timestamp ON trades(token_mint, timestamp DESC);
CREATE INDEX idx_trades_user ON trades(user_wallet);
CREATE INDEX idx_trades_ix_name ON trades(ix_name); 
CREATE INDEX idx_trades_creator ON trades(creator);

-- Indexes for token holders
CREATE INDEX idx_holders_token ON token_holders(token_mint);
CREATE INDEX idx_holders_balance ON token_holders(balance DESC);

-- Indexes for transactions
CREATE INDEX idx_transactions_block_time ON transactions(block_time DESC);
CREATE INDEX idx_transactions_signer ON transactions(signer);
CREATE INDEX idx_transactions_slot ON transactions(slot DESC);
CREATE INDEX idx_transactions_success ON transactions(success);
CREATE INDEX idx_transactions_has_program_data ON transactions(has_program_data);

-- Insert initial stats row
INSERT INTO indexer_stats (id) VALUES (1)
ON CONFLICT (id) DO NOTHING;