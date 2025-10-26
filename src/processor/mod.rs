pub mod calculator;
pub mod state;

use crate::database;
use crate::helius::parser::PumpEvent;
use sqlx::PgPool;
use anyhow::Result;
use tracing::{info, error, debug};

pub async fn process_event(pool: &PgPool, event: PumpEvent) -> Result<()> {
    match event {
        PumpEvent::Create(create) => {
            info!(
                "🆕 New token: {} ({}) - Mint: {}",
                create.name,
                create.symbol,
                create.mint
            );

            // Save to database
            if let Err(e) = database::save_token_creation(pool, &create).await {
                error!("Failed to save token creation: {}", e);
                return Err(e);
            }

            info!("✅ Token saved to database");
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

            if let Err(e) = update_token_metrics(pool, &trade.mint).await {
                error!("Failed to update metrics for {}: {}", trade.mint, e);
            }

            debug!("✅ Trade saved to database");
        }

        PumpEvent::Complete(complete) => {
            info!("🎓 Token graduated to Raydium: {}", complete.mint);

            if let Err(e) = database::mark_token_complete(pool, &complete.mint).await {
                error!("Failed to mark token complete: {}", e);
                return Err(e);
            }

            info!("✅ Token marked as complete");
        }
    }

    Ok(())
}

async fn update_token_metrics(pool: &PgPool, mint: &str) -> Result<()> {

    let latest_trade = sqlx::query!(
        r#"
        SELECT 
            virtual_sol_reserves,
            virtual_token_reserves,
            real_sol_reserves,
            real_token_reserves
        FROM trades
        WHERE token_mint = $1
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
        mint
    )
    .fetch_optional(pool)
    .await?;

    if let Some(trade) = latest_trade {
        // Calculate bonding curve progress
        // Progress = (virtual_sol_reserves / TARGET_SOL) * 100
        const TARGET_SOL: f64 = 85.0; // SOL needed to complete bonding curve
        let sol_reserves = trade.virtual_sol_reserves as f64 / 1_000_000_000.0;
        let progress = (sol_reserves / TARGET_SOL) * 100.0;
        let progress = progress.min(100.0).max(0.0);

        // Calculate market cap (simple estimation)
        let total_supply = 1_000_000_000.0; // 1B tokens
        let token_reserves = trade.virtual_token_reserves as f64 / 1_000_000.0;
        let price_per_token = if token_reserves > 0.0 {
            sol_reserves / token_reserves
        } else {
            0.0
        };
        let market_cap = price_per_token * total_supply;

        // Update in database
        database::update_token_metrics(pool, mint, market_cap, progress).await?;
    }

    Ok(())
}