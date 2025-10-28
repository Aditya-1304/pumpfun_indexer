use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade, Message},
        State, Path,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;
use tracing::{info, error, debug};
use crate::api::AppState;
use redis::AsyncCommands;


pub async fn trades_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_all_trades_socket(socket, state))
}

pub async fn token_trades_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(mint): Path<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_token_trades_socket(socket, state, mint))
}

async fn handle_all_trades_socket(socket: WebSocket, _state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    info!("🔌 New WebSocket client connected: All trades");
    

    let (tx, mut rx) = broadcast::channel::<String>(100);
    
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    
    tokio::spawn(async move {
    
        let client = match redis::Client::open(redis_url.as_str()) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create Redis client: {}", e);
                return;
            }
        };
        
  
        let mut pubsub = match client.get_async_pubsub().await {
            Ok(ps) => ps,
            Err(e) => {
                error!("Failed to get pubsub connection: {}", e);
                return;
            }
        };
        
  
        if let Err(e) = pubsub.subscribe("pump:trades").await {
            error!("Failed to subscribe to Redis: {}", e);
            return;
        }
        
        info!("✅ Subscribed to Redis channel: pump:trades");
        
  
        let mut stream = pubsub.on_message();
        
        loop {
            match stream.next().await {
                Some(msg) => {
                    if let Ok(payload) = msg.get_payload::<String>() {
                        let _ = tx.send(payload);
                    }
                }
                None => {
                    error!("Redis pubsub stream ended");
                    break;
                }
            }
        }
    });
    
    let welcome = serde_json::json!({
        "type": "connected",
        "channel": "pump:trades",
        "message": "Connected to all trades stream"
    });
    
    if sender.send(Message::Text(welcome.to_string().into())).await.is_err() {
        return;
    }
    
    loop {
        tokio::select! {
    
            msg = rx.recv() => {
                match msg {
                    Ok(trade_json) => {
                        if sender.send(Message::Text(trade_json.into())).await.is_err() {
                            debug!("Client disconnected");
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(ping))) => {
                        if sender.send(Message::Pong(ping)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }
    
    info!("🔌 WebSocket client disconnected: All trades");
}

async fn handle_token_trades_socket(socket: WebSocket, _state: AppState, mint: String) {
    let (mut sender, mut receiver) = socket.split();
    
    info!("🔌 New WebSocket client connected: Token {}", mint);
    

    let (tx, mut rx) = broadcast::channel::<String>(100);
    let channel = format!("pump:trades:{}", mint);
    
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    
    let channel_clone = channel.clone();
    tokio::spawn(async move {
        let client = match redis::Client::open(redis_url.as_str()) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create Redis client: {}", e);
                return;
            }
        };
        
        let mut pubsub = match client.get_async_pubsub().await {
            Ok(ps) => ps,
            Err(e) => {
                error!("Failed to get pubsub connection: {}", e);
                return;
            }
        };
        
        if let Err(e) = pubsub.subscribe(&channel_clone).await {
            error!("Failed to subscribe to Redis: {}", e);
            return;
        }
        
        info!("✅ Subscribed to Redis channel: {}", channel_clone);
        
        let mut stream = pubsub.on_message();
        
        loop {
            match stream.next().await {
                Some(msg) => {
                    if let Ok(payload) = msg.get_payload::<String>() {
                        let _ = tx.send(payload);
                    }
                }
                None => {
                    error!("Redis pubsub stream ended");
                    break;
                }
            }
        }
    });
    
    let welcome = serde_json::json!({
        "type": "connected",
        "channel": channel,
        "mint": mint,
        "message": format!("Connected to token trades stream for {}", mint)
    });
    
    if sender.send(Message::Text(welcome.to_string().into())).await.is_err() {
        return;
    }
    
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(trade_json) => {
                        if sender.send(Message::Text(trade_json.into())).await.is_err() {
                            debug!("Client disconnected");
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(ping))) => {
                        if sender.send(Message::Pong(ping)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }
    
    info!("🔌 WebSocket client disconnected: Token {}", mint);
}