mod config;
mod database;
mod helius;
mod processor;
mod storage;
mod api;
mod background;

use anyhow::Result;
use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

#[tokio::main]
async fn main() -> Result<()> {

    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .init();

    info!("🚀 Starting Pump.fun Indexer...");

    let config = config::Config::from_env()?;
    info!("✅ Configuration loaded");
    info!("   Database: {}", mask_db_url(&config.database_url));
    info!("   Redis: {}", config.redis_url);
    info!("   API Port: {}", config.api_port);
    
    if config.coingecko_api_key.is_some() {
        info!("   CoinGecko: Pro API enabled");
    } else {
        info!("   CoinGecko: Free tier (may have rate limits)");
    }

    let pool = database::create_pool(&config.database_url).await?;


    let redis_client = storage::create_redis_client(&config.redis_url).await?;

    let sol_price = Arc::new(RwLock::new(150.0));

    let token_state_map = processor::state::create_state_map();
    info!("✅ In-memory state initialized");


    tokio::spawn(background::start_sol_price_updater(
        sol_price.clone(),
        config.coingecko_api_key.clone(),
    ));
    
    tokio::spawn(background::start_state_backup(pool.clone(), token_state_map.clone()));

    let api_state = api::AppState {
        db: pool.clone(),
        redis: redis_client.clone(),
        token_state: token_state_map.clone(),
        sol_price: sol_price.clone(),
    };
    
    let router = api::create_router(api_state);
    let addr = format!("0.0.0.0:{}", config.api_port);
    
    info!("🌐 Starting API server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let api_server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    info!("✨ Indexer is running!");
    info!("   API: http://localhost:{}", config.api_port);
    info!("   WebSocket: ws://localhost:{}/ws/trades", config.api_port);
    info!("Press Ctrl+C to shutdown");

    let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();
    
    let helius_key = config.helius_api_key.clone();
    let helius_task = tokio::spawn(async move {
        if let Err(e) = helius::start_listener(helius_key, tx_sender).await {
            error!("Helius listener error: {}", e);
        }
    });

    let pool_clone = pool.clone();
    let mut redis_clone = redis_client.clone();
    let state_clone = token_state_map.clone();
    let sol_price_clone = sol_price.clone();
    
    tokio::spawn(async move {
        while let Some(raw_tx) = tx_receiver.recv().await {
            let signature = raw_tx.signature.clone();
            
            let general_tx = raw_tx.to_general_transaction();
            
            if let Err(e) = database::save_general_transaction(&pool_clone, &general_tx).await {
                error!("Failed to save transaction {}: {}", signature, e);
                continue;
            }
            
            match helius::parser::parse_transaction(&signature, &raw_tx.transaction) {
                Ok(events) => {
                    let sol_price_value = *sol_price_clone.read().await;
                    for event in events {
                        if let Err(e) = processor::process_event(
                            &pool_clone,
                            event,
                            &mut redis_clone,
                            &state_clone,
                            sol_price_value,
                        ).await {
                            error!("Failed to process event: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse transaction {}: {}", signature, e);
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    info!("👋 Shutting down gracefully...");

    helius_task.abort();
    api_server.abort();

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