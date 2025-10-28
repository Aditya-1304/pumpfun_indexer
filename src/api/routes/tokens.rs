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
pub struct ListTokensQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    sort: String,
}

fn default_limit() -> i64 { 50 }

#[derive(Debug, Serialize, FromRow)]
pub struct TokenResponse {
    pub mint_address: String,          
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creator_wallet: String,        
    pub market_cap_usd: Option<bigdecimal::BigDecimal>,
    pub bonding_curve_progress: Option<bigdecimal::BigDecimal>,
    pub complete: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_tokens(
    State(state): State<AppState>,
    Query(query): Query<ListTokensQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let limit = query.limit.min(100); 
    let offset = query.offset;
    
    let order_by = match query.sort.as_str() {
        "market_cap" => "market_cap_usd DESC NULLS LAST",
        _ => "created_at DESC",
    };
    
    
    let sql = format!(
        "SELECT mint_address, name, symbol, uri, creator_wallet, 
                market_cap_usd, bonding_curve_progress, complete, created_at
         FROM tokens
         ORDER BY {}
         LIMIT $1 OFFSET $2",
        order_by
    );
    
    let tokens = sqlx::query_as::<_, TokenResponse>(&sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
        })?;
    
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tokens")
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
        })?;
    
    Ok(Json(json!({
        "tokens": tokens,
        "pagination": {
            "total": total.0,
            "limit": limit,
            "offset": offset,
        }
    })))
}

pub async fn get_token(
    State(state): State<AppState>,
    Path(mint): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    
    let state_map = state.token_state.read().await;
    if let Some(token_state) = state_map.get(&mint) {
        return Ok(Json(json!({
            "mint_address": token_state.mint,
            "name": token_state.name,
            "symbol": token_state.symbol,
            "creator": token_state.creator,
            "current_price_sol": token_state.current_price_sol,
            "market_cap_sol": token_state.market_cap_sol,
            "market_cap_usd": token_state.market_cap_usd,
            "bonding_curve_progress": token_state.bonding_curve_progress,
            "complete": token_state.complete,
            "last_updated": token_state.last_updated,
            "source": "in_memory",
        })));
    }
    drop(state_map);
    
    
    let token = sqlx::query_as::<_, TokenResponse>(
        "SELECT mint_address, name, symbol, uri, creator_wallet, 
                market_cap_usd, bonding_curve_progress, complete, created_at
         FROM tokens
         WHERE mint_address = $1"
    )
    .bind(&mint)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;
    
    match token {
        Some(t) => Ok(Json(json!({
            "mint_address": t.mint_address,
            "name": t.name,
            "symbol": t.symbol,
            "creator": t.creator_wallet,
            "market_cap_usd": t.market_cap_usd,
            "bonding_curve_progress": t.bonding_curve_progress,
            "complete": t.complete,
            "created_at": t.created_at,
            "source": "database",
        }))),
        None => Err((StatusCode::NOT_FOUND, "Token not found".to_string())),
    }
}