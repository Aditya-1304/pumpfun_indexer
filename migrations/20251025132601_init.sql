-- Add migration script here
CREATE TABLE IF NOT EXISTS tokens (
    mint_address VARCHAR(44) PRIMARY KEY NOT NULL,
    name VARCHAR(100) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    description TEXT,
    image_uri VARCHAR(255),
    creator_wallet VARCHAR(44) NOT NULL,
    bonding_curve_address VARCHAR(44),
    created_at TIMESTAMPTZ DEFAULT (NOW() AT TIME ZONE 'utc')
);

CREATE TABLE IF NOT EXISTS trades (
    signature VARCHAR(88) PRIMARY KEY NOT NULL,
    token_mint VARCHAR(44) NOT NULL REFERENCES tokens(mint_address),
    trade_type VARCHAR(10) NOT NULL, -- 'buy' or 'sell'
    user_wallet VARCHAR(44) NOT NULL,
    sol_amount DECIMAL(20, 9) NOT NULL,
    token_amount DECIMAL(20, 6) NOT NULL, -- pump.fun tokens have 6 decimals
    timestamp TIMESTAMPTZ NOT NULL
);

-- Add indexes for fast API queries
CREATE INDEX IF NOT EXISTS idx_tokens_creator_wallet ON tokens(creator_wallet);
CREATE INDEX IF NOT EXISTS idx_tokens_created_at ON tokens(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_trades_token_mint_timestamp ON trades(token_mint, timestamp DESC);