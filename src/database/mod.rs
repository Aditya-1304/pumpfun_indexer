pub mod model;
use model::{CreateEvent, TradeEventData, GeneralTransaction};
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;
use chrono::{DateTime, Utc, TimeZone};

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(database_url)
        .await?;

    info!("Database connection established");
    Ok(pool)
}

pub async fn test_connection(pool: &PgPool) -> Result<()> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tokens")
        .fetch_one(pool)
        .await?;
    info!("Database has {} tokens", row.0);
    Ok(())
}


pub async fn save_token_creation(pool: &PgPool, event: &CreateEvent) -> Result<()> {
    let created_at = Utc.timestamp_opt(event.timestamp, 0)
        .single()
        .unwrap_or_else(|| Utc::now());

    sqlx::query!(
        r#"
        INSERT INTO tokens (
            mint_address, 
            name, 
            symbol, 
            uri, 
            bonding_curve_address,
            creator_wallet,
            virtual_token_reserves,
            virtual_sol_reserves,
            real_token_reserves,
            token_total_supply,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (mint_address) DO NOTHING
        "#,
        event.mint,
        event.name,
        event.symbol,
        event.uri,
        event.bonding_curve,
        event.creator,
        event.virtual_token_reserves as i64,
        event.virtual_sol_reserves as i64,
        event.real_token_reserves as i64,
        event.token_total_supply as i64,
        created_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn save_trade(pool: &PgPool, event: &TradeEventData) -> Result<()> {
    let timestamp = Utc.timestamp_opt(event.timestamp, 0)
        .single()
        .unwrap_or_else(|| Utc::now());
    
    let last_update = Utc.timestamp_opt(event.last_update_timestamp, 0)
        .single()
        .unwrap_or_else(|| Utc::now());

    sqlx::query!(
        r#"
        INSERT INTO trades (
            signature,
            token_mint,
            sol_amount,
            token_amount,
            is_buy,
            user_wallet,
            timestamp,
            virtual_sol_reserves,
            virtual_token_reserves,
            real_sol_reserves,
            real_token_reserves,
            fee_recipient,
            fee_basis_points,
            fee,
            creator,
            creator_fee_basis_points,
            creator_fee,
            track_volume,
            total_unclaimed_tokens,
            total_claimed_tokens,
            current_sol_volume,
            last_update_timestamp,
            ix_name
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
        ON CONFLICT (signature) DO NOTHING
        "#,
        event.signature,
        event.mint,
        event.sol_amount as i64,
        event.token_amount as i64,
        event.is_buy,
        event.user,
        timestamp,
        event.virtual_sol_reserves as i64,
        event.virtual_token_reserves as i64,
        event.real_sol_reserves as i64,
        event.real_token_reserves as i64,
        event.fee_recipient,
        event.fee_basis_points as i64,
        event.fee as i64,
        event.creator,
        event.creator_fee_basis_points as i64,
        event.creator_fee as i64,
        event.track_volume,
        event.total_unclaimed_tokens as i64,
        event.total_claimed_tokens as i64,
        event.current_sol_volume as i64,
        last_update,
        event.ix_name
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_token_complete(pool: &PgPool, mint: &str) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE tokens 
        SET complete = true 
        WHERE mint_address = $1
        "#,
        mint
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_token_metrics(
    pool: &PgPool,
    mint: &str,
    market_cap: f64,
    bonding_curve_progress: f64,
) -> Result<()> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let market_cap_bd = BigDecimal::from_str(&market_cap.to_string())?;
    let progress_bd = BigDecimal::from_str(&bonding_curve_progress.to_string())?;

    sqlx::query!(
        r#"
        UPDATE tokens 
        SET 
            market_cap_usd = $2,
            bonding_curve_progress = $3,
            updated_at = NOW()
        WHERE mint_address = $1
        "#,
        mint,
        market_cap_bd,
        progress_bd
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn save_general_transaction(pool: &PgPool, tx: &GeneralTransaction) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO transactions (
            signature, slot, block_time, fee, success, signer,
            instruction_count, log_messages_count, has_program_data,
            accounts_involved, pre_balances, post_balances,
            compute_units_consumed, error_message
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (signature) DO NOTHING
        "#,
        tx.signature,
        tx.slot as i64,
        tx.block_time,
        tx.fee as i64,
        tx.success,
        tx.signer,
        tx.instruction_count,
        tx.log_messages_count,
        tx.has_program_data,
        &tx.accounts_involved,
        &tx.pre_balances,
        &tx.post_balances,
        tx.compute_units_consumed,
        tx.error_message
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update indexer statistics
pub async fn update_stats(
    pool: &PgPool,
    slot: u64,
    tokens_delta: i64,
    trades_delta: i64,
    volume_delta: f64,
) -> Result<()> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let volume_bd = BigDecimal::from_str(&volume_delta.to_string())?;

    sqlx::query!(
        r#"
        UPDATE indexer_stats 
        SET 
            total_transactions = total_transactions + 1,
            total_tokens_created = total_tokens_created + $1,
            total_trades = total_trades + $2,
            total_volume_sol = total_volume_sol + $3,
            last_processed_slot = GREATEST(last_processed_slot, $4),
            last_updated = NOW()
        WHERE id = 1
        "#,
        tokens_delta,
        trades_delta,
        volume_bd,
        slot as i64
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get current indexer statistics
pub async fn get_stats(pool: &PgPool) -> Result<model::IndexerStats> {
    use bigdecimal::BigDecimal;
    
    let stats = sqlx::query!(
        r#"
        SELECT 
            id,
            COALESCE(total_transactions, 0) as "total_transactions!",
            COALESCE(total_tokens_created, 0) as "total_tokens_created!",
            COALESCE(total_trades, 0) as "total_trades!",
            COALESCE(total_volume_sol, 0) as "total_volume_sol!",
            COALESCE(last_processed_slot, 0) as "last_processed_slot!",
            last_updated
        FROM indexer_stats 
        WHERE id = 1
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(model::IndexerStats {
        id: stats.id,
        total_transactions: stats.total_transactions,
        total_tokens_created: stats.total_tokens_created,
        total_trades: stats.total_trades,
        total_volume_sol: stats.total_volume_sol,
        last_processed_slot: stats.last_processed_slot,
        last_updated: stats.last_updated.unwrap_or_else(|| Utc::now()),
    })
}