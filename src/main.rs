mod config;
mod database;

use anyhow::Result;
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

    tracing::info!("Starting Pump.fun Indexer...");

    let config = config::Config::from_env()?;
    tracing::info!("Configuration loaded");
    tracing::info!("Database: {}", mask_db_url(&config.database_url));
    tracing::info!("Redis: {}", config.redis_url);
    tracing::info!("API Port: {}", config.api_port);

    let db_pool = database::create_pool(&config.database_url).await?;

    database::test_connection(&db_pool).await?;

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