pub mod routes;
pub mod handlers;

use axum::{
  Router,
  routing::get,
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use sqlx::PgPool;
use std::sync::Arc;
use crate::processor::state::TokenStateMap;
use crate::storage::RedisClient;


#[derive(Clone)]
pub struct AppState {
  pub db: PgPool,
  pub redis: RedisClient,
  pub token_state: TokenStateMap,
  pub sol_price: Arc<tokio::sync::RwLock<f64>>,
}

pub fn create_router(state: AppState) -> Router {
  Router::new()
    .route("/health", get(handlers::health::health_check))

    .nest("/api", routes::create_api_routes())

    .nest("/ws", routes::create_ws_routes())

    .layer(CorsLayer::new().allow_origin(Any))
    .layer(TraceLayer::new_for_http())

    .with_state(state)
}