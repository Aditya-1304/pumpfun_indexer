use crate::database::model::{CreateEvent, TradeEventData, CompleteEvent};
use anyhow::{Result, anyhow};
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{
  EncodedTransactionWithStatusMeta,
  option_serializer::OptionSerializer,
};
use tracing::{debug, warn};

/// Event discriminators from pump.fun IDL
const CREATE_EVENT_DISCRIMINATOR: [u8; 8] = [27, 114, 169, 77, 222, 235, 99, 118];
const TRADE_EVENT_DISCRIMINATOR: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];
const COMPLETE_EVENT_DISCRIMINATOR: [u8; 8] = [95, 114, 97, 156, 212, 46, 152, 8];


#[derive(Debug, Clone)]
pub enum PumpEvent {
  Create(CreateEvent),
  Trade(TradeEventData),
  Complete(CompleteEvent),
}

pub fn parse_transaction(
  signature: &str,
  transaction: &EncodedTransactionWithStatusMeta,
) -> Result<Vec<PumpEvent>> {
  let mut events = Vec::new();

  let meta = transaction.meta.as_ref()
    .ok_or_else(|| anyhow!("Transaction has no meta"))?;

  if let OptionSerializer::Some(log_messages) = &meta.log_messages {
    for log in log_messages {
      if log.starts_with("Program data: ") {
        if let Some(event) = parse_event_from_log(log, signature) {
          events.push(event);
        }
      }
    }
  }

  if events.is_empty() {
    debug!("No pump.fun events found in transaction {}", signature);
  }

  Ok(events)
}

fn parse_event_from_log(log: &str, signature: &str) -> Option<PumpEvent> {

  let data_str = log.strip_prefix("Program data: ")?;
  let event_data = bs58::decode(data_str).into_vec().ok()?;

  if event_data.len() < 8 {
    return None;
  }

  let discriminator: [u8; 8] = event_data[0..8].try_into().ok()?;

  match discriminator {
    CREATE_EVENT_DISCRIMINATOR => {
      parse_create_event(&event_data[8..]).map(PumpEvent::Create)
    }
    TRADE_EVENT_DISCRIMINATOR => {
      parse_trade_event(&event_data[8..], signature).map(PumpEvent::Trade)
    }
    COMPLETE_EVENT_DISCRIMINATOR => {
      parse_complete_event(&event_data[8..]).map(PumpEvent::Complete)
    }
    _ => {
     
      None
    }
  }
}


fn parse_create_event(data: &[u8]) -> Option<CreateEvent> {
  #[derive(BorshDeserialize)]
  struct CreateEventRaw {
    name: String,
    symbol: String,
    uri: String,
    mint: [u8; 32],
    bonding_curve: [u8; 32],
    user: [u8; 32],
    creator: [u8; 32],
    timestamp: i64,
    virtual_token_reserves: u64,
    virtual_sol_reserves: u64,
    real_token_reserves: u64,
    token_total_supply: u64,
  }

  let raw = CreateEventRaw::deserialize(&mut &data[..]).ok()?;

  Some(CreateEvent {
    name: raw.name,
    symbol: raw.symbol,
    uri: raw.uri,
    mint: Pubkey::new_from_array(raw.mint).to_string(),
    bonding_curve: Pubkey::new_from_array(raw.bonding_curve).to_string(),
    user: Pubkey::new_from_array(raw.user).to_string(),
    creator: Pubkey::new_from_array(raw.creator).to_string(),
    timestamp: raw.timestamp,
    virtual_token_reserves: raw.virtual_token_reserves,
    virtual_sol_reserves: raw.virtual_sol_reserves,
    real_token_reserves: raw.real_token_reserves,
    token_total_supply: raw.token_total_supply,
  })
}


fn parse_trade_event(data: &[u8], signature: &str) -> Option<TradeEventData> {
  #[derive(BorshDeserialize)]
  struct TradeEventRaw {
    mint: [u8; 32],
    sol_amount: u64,
    token_amount: u64,
    is_buy: bool,
    user: [u8; 32],
    timestamp: i64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_sol_reserves: u64,
    real_token_reserves: u64,
    fee_recipient: [u8; 32],
    fee_basis_points: u64,
    fee: u64,
    creator: [u8; 32],
    creator_fee_basis_points: u64,
    creator_fee: u64,
    track_volume: bool,
    total_unclaimed_tokens: u64,
    total_claimed_tokens: u64,
    current_sol_volume: u64,
    last_update_timestamp: i64,
    ix_name: String,
  }

  let raw = TradeEventRaw::deserialize(&mut &data[..]).ok()?;

  Some(TradeEventData {
    mint: Pubkey::new_from_array(raw.mint).to_string(),
    sol_amount: raw.sol_amount,
    token_amount: raw.token_amount,
    is_buy: raw.is_buy,
    user: Pubkey::new_from_array(raw.user).to_string(),
    timestamp: raw.timestamp,
    virtual_sol_reserves: raw.virtual_sol_reserves,
    virtual_token_reserves: raw.virtual_token_reserves,
    real_sol_reserves: raw.real_sol_reserves,
    real_token_reserves: raw.real_token_reserves,
    fee_recipient: Pubkey::new_from_array(raw.fee_recipient).to_string(),
    fee_basis_points: raw.fee_basis_points,
    fee: raw.fee,
    creator: Pubkey::new_from_array(raw.creator).to_string(),
    creator_fee_basis_points: raw.creator_fee_basis_points,
    creator_fee: raw.creator_fee,
    track_volume: raw.track_volume,
    total_unclaimed_tokens: raw.total_unclaimed_tokens,
    total_claimed_tokens: raw.total_claimed_tokens,
    current_sol_volume: raw.current_sol_volume,
    last_update_timestamp: raw.last_update_timestamp,
    ix_name: raw.ix_name,
    signature: signature.to_string(),
  })
}

fn parse_complete_event(data: &[u8]) -> Option<CompleteEvent> {
  #[derive(BorshDeserialize)]
  struct CompleteEventRaw {
    user: [u8; 32],
    mint: [u8; 32],
    bonding_curve: [u8; 32],
    timestamp: i64,
  }

  let raw = CompleteEventRaw::deserialize(&mut &data[..]).ok()?;

  Some(CompleteEvent {
    user: Pubkey::new_from_array(raw.user).to_string(),
    mint: Pubkey::new_from_array(raw.mint).to_string(),
    bonding_curve: Pubkey::new_from_array(raw.bonding_curve).to_string(),
    timestamp: raw.timestamp,
  })
}