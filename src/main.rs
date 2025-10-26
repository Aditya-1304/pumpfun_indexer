mod config;
mod database;
mod helius;

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

    tracing::info!("  Starting Pump.fun Indexer...");

    let config = config::Config::from_env()?;
    tracing::info!("   Configuration loaded");
    tracing::info!("   Database: {}", mask_db_url(&config.database_url));
    tracing::info!("   Redis: {}", config.redis_url);
    tracing::info!("   API Port: {}", config.api_port);

    let db_pool = database::create_pool(&config.database_url).await?;
    database::test_connection(&db_pool).await?;

    let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();

    let api_key = config.helius_api_key.clone();
    tokio::spawn(async move {
        if let Err(e) = helius::start_listener(api_key, tx_sender).await {
            tracing::error!("Helius listener error: {}", e);
        }
    });

    tokio::spawn(async move {
        while let Some(raw_tx) = tx_receiver.recv().await {
            match helius::parser::parse_transaction(&raw_tx.signature, &raw_tx.transaction) {
                Ok(events) => {
                    for event in events {
                        match event {
                            helius::parser::PumpEvent::Create(create) => {
                                tracing::info!("🆕 New token created: {} ({})", create.name, create.symbol);
                            }
                            helius::parser::PumpEvent::Trade(trade) => {
                                let action = if trade.is_buy { "BUY" } else { "SELL" };
                                tracing::info!(
                                    "💰 {} {} tokens for {} SOL",
                                    action,
                                    trade.token_amount as f64 / 1_000_000.0,
                                    trade.sol_amount as f64 / 1_000_000_000.0
                                );
                            }
                            helius::parser::PumpEvent::Complete(complete) => {
                                tracing::info!("🎓 Token graduated: {}", complete.mint);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse transaction {}: {}", raw_tx.signature, e);
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;
   
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