mod config;
mod database;
mod helius;
mod processor;

use anyhow::Result;
use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting Pump.fun Indexer...");

    let config = config::Config::from_env()?;
    tracing::info!("✅ Configuration loaded");
    tracing::info!("   Database: {}", mask_db_url(&config.database_url));
    tracing::info!("   Redis: {}", config.redis_url);
    tracing::info!("   API Port: {}", config.api_port);

    let db_pool = database::create_pool(&config.database_url).await?;
    database::test_connection(&db_pool).await?;

    tracing::info!("✨ Indexer is running!");
    tracing::info!("Press Ctrl+C to shutdown");

    let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();


    let api_key = config.helius_api_key.clone();
    tokio::spawn(async move {
        if let Err(e) = helius::start_listener(api_key, tx_sender).await {
            tracing::error!("Helius listener error: {}", e);
        }
    });

  
    let process_pool = db_pool.clone();
    tokio::spawn(async move {
        let mut tx_count = 0;
        let mut event_count = 0;
        let mut save_count = 0;

        
while let Some(raw_tx) = tx_receiver.recv().await {
    tx_count += 1;

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
                    0, 
                    0, 
                    0.0, 
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
                event_count += events.len();
                
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

                    if let Err(e) = processor::process_event(&process_pool, event).await {
                        tracing::error!("Failed to process event: {}", e);
                    } else {
                        save_count += 1;
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

    
    if tx_count % 50 == 0 {
       
        if let Ok(stats) = database::get_stats(&process_pool).await {
            tracing::info!(
                "📊 Stats: {} txs | {} tokens | {} trades | {:.2} SOL volume",
                stats.total_transactions,
                stats.total_tokens_created,
                stats.total_trades,
                stats.total_volume_sol
            );
        }
    }
}
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("👋 Shutting down gracefully...");
    
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