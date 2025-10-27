pub mod calculator;
pub mod state;
pub mod metrics;

use crate::database;
use crate::helius::parser::PumpEvent;
use crate::storage::RedisClient;
use sqlx::PgPool;
use anyhow::Result;
use tracing::{info, error, debug, warn};
use serde::{Serialize, Deserialize};
use chrono::TimeZone;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeMessage {
    pub signature: String,
    pub mint: String,
    pub is_buy: bool,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub user_wallet: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub market_cap_usd: f64,
    pub price_sol: f64,
}

async fn safe_publish<T: serde::Serialize>(
    redis: &mut RedisClient,
    channel: &str,
    message: &T,
) {
    if let Err(e) = redis.publish(channel, message).await {
        error!("⚠️ Redis publish failed (channel: {}): {}", channel, e);
        error!("   Event will still be saved to database");
    } else {
        debug!("✅ Published to Redis channel: {}", channel);
    }
}

pub async fn process_event(
    pool: &PgPool,
    event: PumpEvent,
    redis: &mut RedisClient,
    state_map: &state::TokenStateMap,
    sol_price_usd: f64,
) -> Result<()> {
    match event {
        PumpEvent::Create(create) => {
            info!(
                "🆕 New token: {} ({}) - Mint: {}",
                create.name,
                create.symbol,
                create.mint
            );

            if let Err(e) = database::save_token_creation(pool, &create).await {
                error!("Failed to save token creation: {}", e);
                return Err(e);
            }

            state::init_token_state(
                state_map,
                create.mint.clone(),
                create.name.clone(),
                create.symbol.clone(),
                create.user.clone(),
                create.virtual_sol_reserves,
                create.virtual_token_reserves,
                create.real_token_reserves,
                create.token_total_supply,
                sol_price_usd,
            ).await;


            let creation_msg = serde_json::json!({
                "mint": create.mint,
                "name": create.name,
                "symbol": create.symbol,
                "creator": create.user,
                "timestamp": create.timestamp,
                "market_cap_sol": 0.0,
            });

            safe_publish(redis, "pump:tokens:new", &creation_msg).await;

            info!("✅ Token saved to database and state initialized");
        }

        PumpEvent::Trade(trade) => {
            let action = if trade.is_buy { "BUY" } else { "SELL" };
            let token_amt = trade.token_amount as f64 / 1_000_000.0;
            let sol_amt = trade.sol_amount as f64 / 1_000_000_000.0;

            info!(
                "💰 {} {:.2} tokens for {:.4} SOL - Mint: {}",
                action, token_amt, sol_amt, trade.mint
            );


            if let Err(e) = database::save_trade(pool, &trade).await {
                error!("Failed to save trade: {}", e);
                return Err(e);
            }

            let updated_state = state::update_token_state(
                state_map,
                &trade.mint,
                trade.virtual_sol_reserves,
                trade.virtual_token_reserves,
                trade.real_sol_reserves,
                trade.real_token_reserves,
                sol_price_usd,
            ).await;


            if let Some(state) = &updated_state {
                if let Err(e) = database::update_token_metrics(
                    pool,
                    &trade.mint,
                    state.market_cap_sol,
                    state.bonding_curve_progress,
                ).await {
                    error!("Failed to update token metrics: {}", e);
                }
            }


            if let Some(state) = updated_state {
                let trade_msg = TradeMessage {
                    signature: trade.signature.clone(),
                    mint: trade.mint.clone(),
                    is_buy: trade.is_buy,
                    sol_amount: trade.sol_amount,
                    token_amount: trade.token_amount,
                    user_wallet: trade.user.clone(),
                    timestamp: chrono::Utc.timestamp_opt(trade.timestamp, 0).unwrap(),
                    market_cap_usd: state.market_cap_usd,
                    price_sol: state.current_price_sol,
                };


                safe_publish(redis, "pump:trades", &trade_msg).await;

                let token_channel = format!("pump:trades:{}", trade.mint);
                safe_publish(redis, &token_channel, &trade_msg).await;
            }

            debug!("✅ Trade processed");
        }

        PumpEvent::Complete(complete) => {
            info!("🎓 Token graduated to Raydium: {}", complete.mint);

            if let Err(e) = database::mark_token_complete(pool, &complete.mint).await {
                error!("Failed to mark token complete: {}", e);
                return Err(e);
            }

            state::mark_token_complete(state_map, &complete.mint).await;

            let completion_msg = serde_json::json!({
                "mint": complete.mint,
                "user": complete.user,
                "timestamp": complete.timestamp,
            });

            safe_publish(redis, "pump:completions", &completion_msg).await;

            info!("✅ Token marked as complete");
        }
    }

    Ok(())
}