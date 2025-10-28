pub mod tokens;
pub mod trades;
pub mod creators;
pub mod stats;
pub mod websocket;

use axum::{
    Router,
    routing::get,
};
use crate::api::AppState;

pub fn create_api_routes() -> Router<AppState> {
    Router::new()
        
        .route("/tokens", get(tokens::list_tokens))
        .route("/tokens/{mint}", get(tokens::get_token))
        
        .route("/tokens/{mint}/trades", get(trades::get_token_trades))
        
        .route("/creators/{wallet}", get(creators::get_creator_tokens))
        

        .route("/stats", get(stats::get_stats))
}


pub fn create_ws_routes() -> Router<AppState> {
    Router::new()

        .route("/trades", get(websocket::trades_websocket))
        .route("/trades/{mint}", get(websocket::token_trades_websocket))
}