use crate::database::model::{CreateEvent, TradeEventData, CompleteEvent};
use anyhow::{Result, anyhow};
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{
  EncodedTransactionWithStatusMeta,
  option_serializer::OptionSerializer,
};
use tracing::{debug, warn, info, error};


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

  debug!("🔍 Parsing transaction: {}", signature);

  let meta = transaction.meta.as_ref()
    .ok_or_else(|| {
      error!("❌ Transaction {} has no meta", signature);
      anyhow!("Transaction has no meta")
    })?;

  debug!("✅ Transaction meta found");

  match &meta.log_messages {
    OptionSerializer::Some(log_messages) => {
      debug!("📋 Found {} log messages", log_messages.len());
      
      let mut program_data_count = 0;
      let mut pump_event_count = 0;
      
      for (idx, log) in log_messages.iter().enumerate() {
        debug!("  Log[{}]: {}", idx, log);
        
        if log.starts_with("Program data: ") {
          program_data_count += 1;
          debug!("🎯 Found 'Program data:' at log index {}", idx);
          
          if let Some(event) = parse_event_from_log(log, signature) {
            pump_event_count += 1;
            info!("✨ Extracted pump.fun event #{} from log index {}", pump_event_count, idx);
            events.push(event);
          } else {
            debug!("⚠️  'Program data:' at index {} is not a pump.fun event", idx);
          }
        }
      }
      
      if program_data_count == 0 {
        debug!("ℹ️  No 'Program data:' logs found in transaction {}", signature);
      } else {
        info!("📊 TX {}: Found {} 'Program data:' logs, {} pump.fun events", 
              signature, program_data_count, pump_event_count);
      }
    }
    OptionSerializer::None => {
      warn!("⚠️  Transaction {} has no log messages", signature);
    }
    OptionSerializer::Skip => {
      warn!("⚠️  Transaction {} log messages were skipped", signature);
    }
  }

  if events.is_empty() {
    debug!("ℹ️  No pump.fun events found in transaction {}", signature);
  } else {
    info!("✅ TX {}: Successfully extracted {} pump.fun events", signature, events.len());
  }

  Ok(events)
}

fn parse_event_from_log(log: &str, signature: &str) -> Option<PumpEvent> {
  debug!("🔎 Attempting to parse event from log");

  let data_str = log.strip_prefix("Program data: ")?;
  debug!("📦 Encoded data: {} (length: {} chars)", 
         if data_str.len() > 50 { 
           format!("{}...", &data_str[..50]) 
         } else { 
           data_str.to_string() 
         }, 
         data_str.len());

  let event_data = if data_str.contains('/') || data_str.contains('+') || data_str.contains('=') {
    debug!("🔧 Detected base64 encoding");
    match base64::decode(data_str) {
      Ok(bytes) => {
        debug!("✅ Decoded {} bytes from base64", bytes.len());
        bytes
      }
      Err(e) => {
        error!("❌ Failed to decode base64: {}", e);
        return None;
      }
    }
  } else {
    debug!("🔧 Detected base58 encoding");
    match bs58::decode(data_str).into_vec() {
      Ok(bytes) => {
        debug!("✅ Decoded {} bytes from base58", bytes.len());
        bytes
      }
      Err(e) => {
        error!("❌ Failed to decode base58: {}", e);
        return None;
      }
    }
  };

  if event_data.len() < 8 {
    warn!("⚠️  Event data too short: {} bytes (need at least 8)", event_data.len());
    return None;
  }

  let discriminator: [u8; 8] = event_data[0..8].try_into().ok()?;
  debug!("🔑 Discriminator: {:?}", discriminator);

  match discriminator {
    CREATE_EVENT_DISCRIMINATOR => {
      info!("🎉 CREATE event discriminator matched!");
      match parse_create_event(&event_data[8..]) {
        Some(event) => {
          info!("✅ Successfully parsed CREATE event: token={}, symbol={}", 
                event.mint, event.symbol);
          Some(PumpEvent::Create(event))
        }
        None => {
          error!("❌ Failed to deserialize CREATE event data");
          None
        }
      }
    }
    TRADE_EVENT_DISCRIMINATOR => {
      info!("💰 TRADE event discriminator matched!");
      match parse_trade_event(&event_data[8..], signature) {
        Some(event) => {
          info!("✅ Successfully parsed TRADE event: {} {} tokens for {} SOL", 
                if event.is_buy { "BUY" } else { "SELL" },
                event.token_amount as f64 / 1_000_000.0,
                event.sol_amount as f64 / 1_000_000_000.0);
          Some(PumpEvent::Trade(event))
        }
        None => {
          error!("❌ Failed to deserialize TRADE event data");
          None
        }
      }
    }
    COMPLETE_EVENT_DISCRIMINATOR => {
      info!("🏁 COMPLETE event discriminator matched!");
      match parse_complete_event(&event_data[8..]) {
        Some(event) => {
          info!("✅ Successfully parsed COMPLETE event: token={}", event.mint);
          Some(PumpEvent::Complete(event))
        }
        None => {
          error!("❌ Failed to deserialize COMPLETE event data");
          None
        }
      }
    }
    _ => {
      debug!("❓ Unknown discriminator: {:?} (not a pump.fun event)", discriminator);
      None
    }
  }
}

fn parse_create_event(data: &[u8]) -> Option<CreateEvent> {
  debug!("🔧 Parsing CREATE event from {} bytes", data.len());

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

  let raw = match CreateEventRaw::deserialize(&mut &data[..]) {
    Ok(r) => {
      debug!("✅ Borsh deserialization successful");
      r
    }
    Err(e) => {
      error!("❌ Borsh deserialization failed: {}", e);
      error!("   Data length: {} bytes", data.len());
      error!("   First 32 bytes: {:?}", &data[..data.len().min(32)]);
      return None;
    }
  };

  debug!("📝 CREATE event details:");
  debug!("   Name: {}", raw.name);
  debug!("   Symbol: {}", raw.symbol);
  debug!("   Mint: {}", Pubkey::new_from_array(raw.mint));
  debug!("   Creator: {}", Pubkey::new_from_array(raw.creator));

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
  debug!("🔧 Parsing TRADE event from {} bytes", data.len());

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

  let raw = match TradeEventRaw::deserialize(&mut &data[..]) {
    Ok(r) => {
      debug!("✅ Borsh deserialization successful");
      r
    }
    Err(e) => {
      error!("❌ Borsh deserialization failed: {}", e);
      error!("   Data length: {} bytes", data.len());
      error!("   First 32 bytes: {:?}", &data[..data.len().min(32)]);
      return None;
    }
  };

  debug!("📝 TRADE event details:");
  debug!("   Type: {}", if raw.is_buy { "BUY" } else { "SELL" });
  debug!("   Mint: {}", Pubkey::new_from_array(raw.mint));
  debug!("   User: {}", Pubkey::new_from_array(raw.user));
  debug!("   SOL: {} lamports", raw.sol_amount);
  debug!("   Tokens: {}", raw.token_amount);

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
  debug!("🔧 Parsing COMPLETE event from {} bytes", data.len());

  #[derive(BorshDeserialize)]
  struct CompleteEventRaw {
    user: [u8; 32],
    mint: [u8; 32],
    bonding_curve: [u8; 32],
    timestamp: i64,
  }

  let raw = match CompleteEventRaw::deserialize(&mut &data[..]) {
    Ok(r) => {
      debug!("✅ Borsh deserialization successful");
      r
    }
    Err(e) => {
      error!("❌ Borsh deserialization failed: {}", e);
      error!("   Data length: {} bytes", data.len());
      error!("   First 32 bytes: {:?}", &data[..data.len().min(32)]);
      return None;
    }
  };

  debug!("📝 COMPLETE event details:");
  debug!("   Mint: {}", Pubkey::new_from_array(raw.mint));
  debug!("   User: {}", Pubkey::new_from_array(raw.user));

  Some(CompleteEvent {
    user: Pubkey::new_from_array(raw.user).to_string(),
    mint: Pubkey::new_from_array(raw.mint).to_string(),
    bonding_curve: Pubkey::new_from_array(raw.bonding_curve).to_string(),
    timestamp: raw.timestamp,
  })
}