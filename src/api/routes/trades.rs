use axum::{
    extract::{State, Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct TradesQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 { 50 }

#[derive(Debug, Serialize, FromRow)]
pub struct TradeResponse {
    pub signature: String,
    pub mint: String,
    pub trader_wallet: String,
    pub is_buy: bool,
    pub sol_amount: i64,
    pub token_amount: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub async fn get_token_trades(
    State(state): State<AppState>,
    Path(mint): Path<String>,
    Query(query): Query<TradesQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let limit = query.limit.min(100);
    let offset = query.offset;
    
    let trades = sqlx::query_as::<_, TradeResponse>(
        "SELECT signature, mint, trader_wallet, is_buy, sol_amount, token_amount, timestamp
         FROM trades
         WHERE mint = $1
         ORDER BY timestamp DESC
         LIMIT $2 OFFSET $3"
    )
    .bind(&mint)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;
    
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM trades WHERE mint = $1"
    )
    .bind(&mint)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;
    
    Ok(Json(json!({
        "trades": trades,
        "pagination": {
            "total": total.0,
            "limit": limit,
            "offset": offset,
        }
    })))
}