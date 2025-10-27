mod config;
mod database;
mod helius;
mod processor;
mod storage;

use anyhow::Result;
use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("  Starting Pump.fun Indexer...");

    let config = config::Config::from_env()?;
    tracing::info!("   Configuration loaded");
    tracing::info!("   Database: {}", mask_db_url(&config.database_url));
    tracing::info!("   Redis: {}", config.redis_url);
    tracing::info!("   API Port: {}", config.api_port);

    
    let db_pool = database::create_pool(&config.database_url).await?;
    database::test_connection(&db_pool).await?;

    
    let mut redis_client = storage::create_redis_client(&config.redis_url).await?;

    
    let token_state = processor::state::create_state_map();
    tracing::info!("  In-memory state initialized");

    
    let sol_price_usd = Arc::new(tokio::sync::RwLock::new(100.0));

    tracing::info!("  Indexer is running!");
    tracing::info!("Press Ctrl+C to shutdown");

    let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();

    let api_key = config.helius_api_key.clone();
    tokio::spawn(async move {
        if let Err(e) = helius::start_listener(api_key, tx_sender).await {
            tracing::error!("Helius listener error: {}", e);
        }
    });

    let process_pool = db_pool.clone();
    let process_state = token_state.clone();
    let sol_price_arc = sol_price_usd.clone();
    
    tokio::spawn(async move {
        while let Some(raw_tx) = tx_receiver.recv().await {
            let block_time = None;

            match helius::extractor::extract_transaction_metadata(
                &raw_tx.signature,
                raw_tx.slot,
                &raw_tx.transaction,
                block_time,
            ) {
                Ok(general_tx) => {
                    if let Err(e) = database::save_general_transaction(&process_pool, &general_tx).await {
                        tracing::error!("Failed to save transaction metadata: {}", e);
                    } else {

                        if let Err(e) = database::update_stats(
                            &process_pool,
                            raw_tx.slot,
                            0, 0, 0.0,
                        ).await {
                            tracing::error!("Failed to update stats: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to extract transaction metadata: {}", e);
                }
            }

            match helius::parser::parse_transaction(&raw_tx.signature, &raw_tx.transaction) {
                Ok(events) => {
                    if !events.is_empty() {
                        let mut tokens_created = 0i64;
                        let mut trades_made = 0i64;
                        let mut volume_sol = 0.0f64;

                        for event in events {
                            match &event {
                                helius::parser::PumpEvent::Create(_) => tokens_created += 1,
                                helius::parser::PumpEvent::Trade(trade) => {
                                    trades_made += 1;
                                    volume_sol += trade.sol_amount as f64 / 1_000_000_000.0;
                                }
                                helius::parser::PumpEvent::Complete(_) => {}
                            }

                            let current_sol_price = *sol_price_arc.read().await;

                            if let Err(e) = processor::process_event(
                                &process_pool,
                                event,
                                &mut redis_client,
                                &process_state,
                                current_sol_price,
                            ).await {
                                tracing::error!("Failed to process event: {}", e);
                            }
                        }

                        if let Err(e) = database::update_stats(
                            &process_pool,
                            raw_tx.slot,
                            tokens_created,
                            trades_made,
                            volume_sol,
                        ).await {
                            tracing::error!("Failed to update stats: {}", e);
                        }
                    }
                }
                Err(_) => {
      
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("   Shutting down gracefully...");
    
    Ok(())
}

fn mask_db_url(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            masked.replace_range(colon_pos + 1..at_pos, "****");
            return masked;
        }
    }
    url.to_string()
}