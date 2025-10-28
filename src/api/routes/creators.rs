use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::FromRow;
use crate::api::AppState;

#[derive(Debug, Serialize, FromRow)]
pub struct CreatorTokenResponse {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub market_cap_sol: f64,
    pub complete: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_creator_tokens(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let tokens = sqlx::query_as::<_, CreatorTokenResponse>(
        "SELECT mint, name, symbol, market_cap_sol, complete, created_at
         FROM tokens
         WHERE creator = $1
         ORDER BY created_at DESC"
    )
    .bind(&wallet)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;
    
    Ok(Json(json!({
        "creator": wallet,
        "tokens": tokens,
        "total": tokens.len(),
    })))
}