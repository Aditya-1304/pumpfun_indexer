use sqlx::postgres::PgPoolOptions;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    println!("🔍 Checking database contents...\n");
    
    // Check transactions
    let tx_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
        .fetch_one(&pool)
        .await?;
    println!("📝 Total Transactions: {}", tx_count.0);
    
    // Check tokens
    let token_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tokens")
        .fetch_one(&pool)
        .await?;
    println!("🪙 Total Tokens: {}", token_count.0);
    
    // Check trades
    let trade_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM trades")
        .fetch_one(&pool)
        .await?;
    println!("💰 Total Trades: {}", trade_count.0);
    
    // Check completed tokens
    let complete_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM tokens WHERE complete = true"
    )
    .fetch_one(&pool)
    .await?;
    println!("✅ Completed Tokens: {}", complete_count.0);
    
    // Latest token
    let latest_token = sqlx::query!(
        "SELECT mint_address, name, symbol, creator_wallet, created_at 
         FROM tokens 
         ORDER BY created_at DESC 
         LIMIT 1"
    )
    .fetch_optional(&pool)
    .await?;
    
    if let Some(token) = latest_token {
        println!("\n🆕 Latest Token:");
        println!("   Mint: {}", token.mint_address);
        println!("   Name: {}", token.name);
        println!("   Symbol: {}", token.symbol);
        println!("   Creator: {}", token.creator_wallet);
        println!("   Created: {}", token.created_at);
    }
    
    // Latest trade
    let latest_trade = sqlx::query!(
        "SELECT signature, token_mint, user_wallet, is_buy, sol_amount, timestamp
         FROM trades
         ORDER BY timestamp DESC
         LIMIT 1"
    )
    .fetch_optional(&pool)
    .await?;
    
    if let Some(trade) = latest_trade {
        println!("\n💸 Latest Trade:");
        println!("   Signature: {}", trade.signature);
        println!("   Token: {}", trade.token_mint);
        println!("   User: {}", trade.user_wallet);
        println!("   Type: {}", if trade.is_buy { "BUY" } else { "SELL" });
        println!("   SOL: {:.4}", trade.sol_amount as f64 / 1_000_000_000.0);
        println!("   Time: {}", trade.timestamp);
    }
    
    // Indexer stats
    let stats = sqlx::query!(
        "SELECT total_transactions, total_tokens_created, total_trades, 
                total_volume_sol, last_processed_slot, last_updated
         FROM indexer_stats
         WHERE id = 1"
    )
    .fetch_optional(&pool)
    .await?;
    
    if let Some(stats) = stats {
        println!("\n📊 Indexer Stats:");
        println!("   Total TXs: {}", stats.total_transactions.unwrap_or(0));
        println!("   Total Tokens: {}", stats.total_tokens_created.unwrap_or(0));
        println!("   Total Trades: {}", stats.total_trades.unwrap_or(0));
        let vol = stats.total_volume_sol
            .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
        println!("   Volume: {:.4} SOL", vol / 1_000_000_000.0);
        println!("   Last Slot: {}", stats.last_processed_slot.unwrap_or(0));
        println!("   Updated: {}", stats.last_updated.map(|t| t.to_string()).unwrap_or_else(|| "N/A".to_string()));
    }
    
    Ok(())
}