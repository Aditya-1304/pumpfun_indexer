use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use bigdecimal::BigDecimal;

/// Token from CreateEvent (12 fields)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Token {
  pub mint_address: String,
  pub name: String,
  pub symbol: String,
  pub bonding_curve_address: String,
  pub creator_wallet: String,

  pub virtual_token_reserves: i64,
  pub virtual_sol_reserves: i64,
  pub real_token_reserves: i64,
  pub token_total_supply: i64,

  pub market_cap_usd: Option<BigDecimal>,
  pub bonding_curve_progress: Option<BigDecimal>,

  pub complete: bool,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

/// Trade from TradeEvent
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Trade {
  pub signature: String,
  pub token_mint: String,

  pub sol_amount: i64,
  pub token_amount: i64,
  pub is_buy: bool,
  pub user_wallet: String,
  pub timestamp: DateTime<Utc>,

  pub virtual_sol_reserves: i64,
  pub virtual_token_reserves: i64,
  pub real_sol_reserves: i64,
  pub real_token_reserves: i64,

  pub fee_recipient: String,
  pub fee_basis_points: i64,
  pub fee: i64,

  pub creator: String,
  pub creator_fee_basis_points: i64,
  pub creator_fee: i64,

  pub track_volume: bool,
  pub total_unclaimed_tokens: i64, 
  pub total_claimed_tokens: i64, 
  pub current_sol_volume: i64,
  pub last_update_timestamp: DateTime<Utc>,

  pub ix_name: String,
  pub price_usd: Option<BigDecimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenHolder {
  pub id: i32,
  pub token_mint: String,
  pub user_wallet: String,
  pub balance: i64,
  pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralTransaction {
  pub signature: String,
  pub slot: u64,
  pub block_time: DateTime<Utc>,
  pub fee: u64,
  pub success: bool,
  pub signer: String,
  pub instruction_count: i32,
  pub log_messages_count: i32,
  pub has_program_data: bool,
  pub accounts_involved: Vec<String>,
  pub pre_balances: Vec<i64>,
  pub post_balances: Vec<i64>,
  pub compute_units_consumed: Option<i64>,
  pub error_message: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IndexerStats {
  pub id: i32,
  pub total_transactions: i64,
  pub total_tokens_created: i64,
  pub total_trades: i64,
  pub total_volume_sol: BigDecimal,
  pub last_processed_slot: i64,
  pub last_updated: DateTime<Utc>,
}

/// CreateEvent
#[derive(Debug, Clone)]
pub struct CreateEvent {
  pub name: String,
  pub symbol: String,
  pub uri: String,
  pub mint: String,
  pub bonding_curve: String,
  pub user: String,
  pub creator: String,
  pub timestamp: i64,
  pub virtual_token_reserves: u64,
  pub virtual_sol_reserves: u64,
  pub real_token_reserves: u64,
  pub token_total_supply: u64,
}

/// TradeEvent
#[derive(Debug, Clone)]
pub struct TradeEventData {
  pub mint: String,
  pub sol_amount: u64,
  pub token_amount: u64,
  pub is_buy: bool,
  pub user: String,
  pub timestamp: i64,
  pub virtual_sol_reserves: u64,
  pub virtual_token_reserves: u64,
  pub real_sol_reserves: u64,
  pub real_token_reserves: u64,
  pub fee_recipient: String,
  pub fee_basis_points: u64,
  pub fee: u64,
  pub creator: String,
  pub creator_fee_basis_points: u64,
  pub creator_fee: u64,
  pub track_volume: bool,
  pub total_unclaimed_tokens: u64,
  pub total_claimed_tokens: u64,
  pub current_sol_volume: u64,
  pub last_update_timestamp: i64,
  pub ix_name: String,
  pub signature: String,
}

/// CompleteEvent
#[derive(Debug, Clone)]
pub struct CompleteEvent {
  pub user: String,
  pub mint: String,
  pub bonding_curve: String,
  pub timestamp: i64,
}