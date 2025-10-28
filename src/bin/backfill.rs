use anyhow::{Result, Context};
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::{pubkey::Pubkey, signature::Signature, commitment_config::CommitmentConfig};
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn, error};
use sqlx::postgres::PgPoolOptions;
use chrono::TimeZone;

const PUMP_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

#[derive(Parser, Debug)]
#[command(name = "backfill")]
#[command(about = "Backfill historical pump.fun transactions", long_about = None)]
struct Args {
    #[arg(long)]
    before: Option<String>,
    
    #[arg(long, default_value = "1000")]
    batch_size: usize,
    
    #[arg(long)]
    max_txs: Option<usize>,
    
    #[arg(long, default_value = "100")]
    delay_ms: u64,
    
    #[arg(long)]
    tokens_only: bool,
    
    #[arg(long)]
    trades_only: bool,
    
    #[arg(long, default_value = "10")]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<()> {

    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();
    
    let args = Args::parse();
    

    if args.tokens_only && args.trades_only {
        error!("❌ Cannot use --tokens-only and --trades-only together");
        std::process::exit(1);
    }
    
    info!("🚀 Starting Pump.fun Backfill Tool");
    info!("   Batch size: {}", args.batch_size);
    info!("   Concurrency: {}", args.concurrency);
    
    if args.tokens_only {
        info!("   📍 MODE: PHASE 1 - TOKENS ONLY");
        info!("   Will collect: Token creations");
        info!("   Will skip: Trades, Completions");
    } else if args.trades_only {
        info!("   📍 MODE: PHASE 2 - TRADES ONLY");
        info!("   Will collect: Trades, Completions");
        info!("   Will skip: Token creations");
    } else {
        info!("   📍 MODE: FULL BACKFILL");
        info!("   Will collect: Everything (not recommended - use two-phase)");
    }
    
    if let Some(max) = args.max_txs {
        info!("   Max transactions: {}", max);
    }
    
    dotenv::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?;
    let helius_api_key = std::env::var("HELIUS_API_KEY")
        .context("HELIUS_API_KEY must be set")?;
    
    info!("📊 Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .context("Failed to connect to database")?;
    
    info!("✅ Database connected");
    
  
    let rpc_url = format!("https://mainnet.helius-rpc.com/?api-key={}", helius_api_key);
    let client = RpcClient::new_with_timeout(rpc_url, Duration::from_secs(60));
    
    info!("🔗 RPC client connected to Helius");
    
    let pump_pubkey = Pubkey::from_str(PUMP_PROGRAM)?;
    
    let mut before_sig = if let Some(sig_str) = args.before {
        Some(Signature::from_str(&sig_str)?)
    } else {
        None
    };
    
  
    let mut total_processed = 0;
    let mut total_events = 0;
    let mut total_tokens = 0;
    let mut total_trades = 0;
    let mut total_completions = 0;
    let mut batch_count = 0;
    let mut skipped_txs = 0;
    let mut foreign_key_errors = 0; // Track trades without tokens
    
    let start_time = std::time::Instant::now();
    
    info!("🔍 Starting signature fetch...");
    
    loop {
        batch_count += 1;
        let batch_start = std::time::Instant::now();
        
        info!("📡 Batch #{}: Fetching up to {} signatures...", batch_count, args.batch_size);
        
  
        let sigs = match client.get_signatures_for_address_with_config(
            &pump_pubkey,
            solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config {
                before: before_sig,
                limit: Some(args.batch_size),
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
        ) {
            Ok(s) => s,
            Err(e) => {
                error!("❌ Failed to fetch signatures: {}", e);
                warn!("   Retrying in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        
        if sigs.is_empty() {
            info!("✅ No more signatures to fetch - reached the beginning!");
            break;
        }
        
        info!("📥 Processing batch of {} signatures...", sigs.len());
        
        let chunk_size = 100;
        for (chunk_idx, chunk) in sigs.chunks(chunk_size).enumerate() {
            info!("   Processing chunk {}/{} ({} sigs)...", 
                  chunk_idx + 1, 
                  (sigs.len() + chunk_size - 1) / chunk_size,
                  chunk.len());
            
            for sig_info in chunk {
                let sig = match Signature::from_str(&sig_info.signature) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("⚠️  Invalid signature format: {}", e);
                        continue;
                    }
                };
                
        
                if sig_info.err.is_some() {
                    skipped_txs += 1;
                    continue;
                }
                
        
                let tx_config = RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::JsonParsed),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                };
                
                let confirmed_tx = match client.get_transaction_with_config(&sig, tx_config) {
                    Ok(tx) => tx,
                    Err(e) => {
                        warn!("⚠️  Failed to fetch TX {}: {}", sig, e);
                        skipped_txs += 1;
                        continue;
                    }
                };
                
                
                match pumpfun_indexer::helius::parser::parse_transaction(
                    &sig_info.signature, 
                    &confirmed_tx.transaction
                ) {
                    Ok(events) => {
                        if !events.is_empty() {
                            total_events += events.len();
                            
                            for event in events {
                                match event {
                                    pumpfun_indexer::helius::parser::PumpEvent::Create(create) => {
                
                                        if args.trades_only {
                                            continue;
                                        }
                                        
                                        if let Err(e) = save_create_event(&pool, &create).await {
                                            if !e.to_string().contains("duplicate key") {
                                                error!("❌ Failed to save CREATE: {}", e);
                                            }
                                        } else {
                                            total_tokens += 1;
                                            if total_tokens % 50 == 0 {
                                                info!("      ✨ {} tokens created so far", total_tokens);
                                            }
                                        }
                                    }
                                    pumpfun_indexer::helius::parser::PumpEvent::Trade(trade) => {
                                        
                                        if args.tokens_only {
                                            continue; 
                                        }
                                        
                                        if let Err(e) = save_trade_event(&pool, &trade).await {
                                            let err_str = e.to_string();
                                            
                                          
                                            if err_str.contains("foreign key") || err_str.contains("violates") {
                                                foreign_key_errors += 1;
                                                if foreign_key_errors % 100 == 1 {
                                                    warn!("⚠️  {} trades skipped (token not found in DB)", foreign_key_errors);
                                                }
                                            } else if !err_str.contains("duplicate key") {
                                                error!("❌ Failed to save TRADE: {}", e);
                                            }
                                        } else {
                                            total_trades += 1;
                                            if total_trades % 1000 == 0 {
                                                info!("      💰 {} trades saved so far", total_trades);
                                            }
                                        }
                                    }
                                    pumpfun_indexer::helius::parser::PumpEvent::Complete(complete) => {
                                        
                                        if args.tokens_only {
                                            continue; 
                                        }
                                        
                                        if let Err(e) = mark_complete(&pool, &complete.mint).await {
                                            if !e.to_string().contains("duplicate key") {
                                                error!("❌ Failed to mark COMPLETE: {}", e);
                                            }
                                        } else {
                                            total_completions += 1;
                                            if total_completions % 10 == 0 {
                                                info!("      🎓 {} tokens graduated so far", total_completions);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        if !err_str.contains("base58") && !err_str.contains("base64") {
                            warn!("⚠️  Failed to parse TX {}: {}", &sig_info.signature[..8], e);
                        }
                    }
                }
                
                total_processed += 1;
                
                if let Some(max) = args.max_txs {
                    if total_processed >= max {
                        info!("✅ Reached max transactions limit");
                        break;
                    }
                }
            }
            
            if let Some(max) = args.max_txs {
                if total_processed >= max {
                    break;
                }
            }
        }
        
        // Batch summary
        let batch_elapsed = batch_start.elapsed();
        let total_elapsed = start_time.elapsed();
        let tx_per_sec = total_processed as f64 / total_elapsed.as_secs_f64();
        
        info!("📊 Batch #{} complete ({:.1}s):", batch_count, batch_elapsed.as_secs_f64());
        info!("   Processed: {}/{} TXs", total_processed, total_processed + skipped_txs);
        info!("   Events: {} ({} tokens, {} trades, {} completions)", 
              total_events, total_tokens, total_trades, total_completions);
        
        if args.trades_only && foreign_key_errors > 0 {
            info!("   Foreign key errors: {} (run --tokens-only first)", foreign_key_errors);
        }
        
        info!("   Speed: {:.2} TX/sec | Elapsed: {:?}", tx_per_sec, total_elapsed);
        
        if let Some(max) = args.max_txs {
            if total_processed >= max {
                break;
            }
        }
        
        before_sig = Some(Signature::from_str(&sigs.last().unwrap().signature)?);
        
        if args.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(args.delay_ms)).await;
        }
    }
    
    let total_time = start_time.elapsed();
    let avg_speed = total_processed as f64 / total_time.as_secs_f64();
    
    info!("");
    info!("🎉 Backfill Complete!");
    info!("════════════════════════════════════");
    info!("📊 Final Statistics:");
    info!("   Transactions processed: {}", total_processed);
    info!("   Transactions skipped: {}", skipped_txs);
    info!("   Total events: {}", total_events);
    info!("   ├─ Tokens created: {}", total_tokens);
    info!("   ├─ Trades: {}", total_trades);
    info!("   └─ Completions: {}", total_completions);
    
    if args.trades_only && foreign_key_errors > 0 {
        warn!("   ⚠️  Foreign key errors: {} trades skipped (tokens not in DB)", foreign_key_errors);
        warn!("   Run PHASE 1 (--tokens-only) first to fix this!");
    }
    
    info!("   Total time: {:?}", total_time);
    info!("   Average speed: {:.2} TX/sec", avg_speed);
    info!("════════════════════════════════════");
    
    Ok(())
}

async fn save_create_event(
    pool: &sqlx::PgPool,
    event: &pumpfun_indexer::database::model::CreateEvent,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO tokens (
            mint_address, name, symbol, uri, creator_wallet, bonding_curve_address,
            virtual_sol_reserves, virtual_token_reserves, real_token_reserves,
            token_total_supply, complete, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (mint_address) DO UPDATE SET
            name = EXCLUDED.name,
            symbol = EXCLUDED.symbol,
            uri = EXCLUDED.uri,
            creator_wallet = EXCLUDED.creator_wallet,
            bonding_curve_address = EXCLUDED.bonding_curve_address,
            virtual_sol_reserves = EXCLUDED.virtual_sol_reserves,
            virtual_token_reserves = EXCLUDED.virtual_token_reserves,
            real_token_reserves = EXCLUDED.real_token_reserves,
            token_total_supply = EXCLUDED.token_total_supply,
            updated_at = NOW()
        "
    )
    .bind(&event.mint)
    .bind(&event.name)
    .bind(&event.symbol)
    .bind(&event.uri)
    .bind(&event.creator)
    .bind(&event.bonding_curve)
    .bind(event.virtual_sol_reserves as i64)
    .bind(event.virtual_token_reserves as i64)
    .bind(event.real_token_reserves as i64)
    .bind(event.token_total_supply as i64)
    .bind(false)
    .bind(chrono::Utc.timestamp_opt(event.timestamp, 0).unwrap())
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn save_trade_event(
    pool: &sqlx::PgPool,
    event: &pumpfun_indexer::database::model::TradeEventData,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO trades (
            signature, token_mint, user_wallet, is_buy,
            sol_amount, token_amount, timestamp,
            virtual_sol_reserves, virtual_token_reserves,
            real_sol_reserves, real_token_reserves,
            fee_recipient, fee_basis_points, fee,
            creator, creator_fee_basis_points, creator_fee,
            track_volume, total_unclaimed_tokens, total_claimed_tokens,
            current_sol_volume, last_update_timestamp, ix_name
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
        ON CONFLICT (signature) DO NOTHING"
    )
    .bind(&event.signature)
    .bind(&event.mint)
    .bind(&event.user)
    .bind(event.is_buy)
    .bind(event.sol_amount as i64)
    .bind(event.token_amount as i64)
    .bind(chrono::Utc.timestamp_opt(event.timestamp, 0).unwrap())
    .bind(event.virtual_sol_reserves as i64)
    .bind(event.virtual_token_reserves as i64)
    .bind(event.real_sol_reserves as i64)
    .bind(event.real_token_reserves as i64)
    .bind(&event.fee_recipient)
    .bind(event.fee_basis_points as i64)
    .bind(event.fee as i64)
    .bind(&event.creator)
    .bind(event.creator_fee_basis_points as i64)
    .bind(event.creator_fee as i64)
    .bind(event.track_volume)
    .bind(event.total_unclaimed_tokens as i64)
    .bind(event.total_claimed_tokens as i64)
    .bind(event.current_sol_volume as i64)
    .bind(chrono::Utc.timestamp_opt(event.last_update_timestamp, 0).unwrap())
    .bind(&event.ix_name)
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn mark_complete(pool: &sqlx::PgPool, mint: &str) -> Result<()> {
    sqlx::query("UPDATE tokens SET complete = true WHERE mint_address = $1")
        .bind(mint)
        .execute(pool)
        .await?;
    
    Ok(())
}