use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use crate::api::AppState;

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let stats = sqlx::query!(
        "SELECT 
            total_transactions, 
            total_tokens_created, 
            total_trades, 
            total_volume_sol, 
            last_processed_slot,  -- 🔥 ADD: Show last indexed slot
            last_updated
         FROM indexer_stats
         WHERE id = 1"
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;
    
    let sol_price = *state.sol_price.read().await;
    
    if let Some(stats) = stats {
        let volume_sol = stats.total_volume_sol
            .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
        
        Ok(Json(json!({
            "total_transactions": stats.total_transactions,
            "total_tokens_created": stats.total_tokens_created,
            "total_trades": stats.total_trades,
            "total_volume_sol": volume_sol,
            "total_volume_usd": volume_sol * sol_price,
            "sol_price_usd": sol_price,
            "last_processed_slot": stats.last_processed_slot,
            "last_updated": stats.last_updated,
        })))
    } else {
        Ok(Json(json!({
            "total_transactions": 0,
            "total_tokens_created": 0,
            "total_trades": 0,
            "total_volume_sol": 0.0,
            "total_volume_usd": 0.0,
            "sol_price_usd": sol_price,
            "last_processed_slot": 0,
        })))
    }
}