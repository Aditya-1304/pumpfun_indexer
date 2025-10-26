pub mod parser;
pub mod extractor;

use tokio::sync::Semaphore;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_transaction_status::EncodedTransactionWithStatusMeta;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use std::str::FromStr;
use std::time::Duration;

const PUMP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

#[derive(Debug)]
pub struct RawTransaction {
    pub signature: String,
    pub slot: u64,
    pub transaction: EncodedTransactionWithStatusMeta,
}


#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}


#[derive(Debug, Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    #[serde(default)]
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}


#[derive(Debug, Deserialize)]
struct LogsNotification {
    subscription: u64,
    result: LogsNotificationResult,
}

#[derive(Debug, Deserialize)]
struct LogsNotificationResult {
    context: LogsContext,
    value: LogsValue,
}

#[derive(Debug, Deserialize)]
struct LogsContext {
    slot: u64,
}

#[derive(Debug, Deserialize)]
struct LogsValue {
    signature: String,
    #[serde(default)]
    err: Option<serde_json::Value>,
    logs: Vec<String>,
}

pub async fn start_listener(
    api_key: String,
    tx_sender: mpsc::UnboundedSender<RawTransaction>,
) -> Result<()> {
    info!("Connecting to Helius WebSocket...");
    
    let ws_url = format!("wss://mainnet.helius-rpc.com/?api-key={}", api_key);
    info!("   URL: {}...{}", &ws_url[..50], &ws_url[ws_url.len()-4..]);

    let (ws_stream, response) = connect_async(&ws_url).await
        .map_err(|e| {
            error!("Failed to connect to WebSocket: {}", e);
            anyhow!("WebSocket connection failed: {}", e)
        })?;

    info!("WebSocket connected!");
    info!("Response status: {}", response.status());

    let (mut write, mut read) = ws_stream.split();

    let subscribe_request = RpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "logsSubscribe".to_string(),
        params: json!([
            {
                "mentions": [PUMP_PROGRAM_ID]
            },
            {
                "commitment": "confirmed"
            }
        ]),
    };

    let subscribe_msg = serde_json::to_string(&subscribe_request)?;
    
    info!("📡 Subscribing to pump.fun program logs: {}", PUMP_PROGRAM_ID);
    
    write.send(Message::Text(subscribe_msg.into())).await
        .map_err(|e| anyhow!("Failed to send subscription: {}", e))?;

    info!("Subscription request sent");
    info!("Listening for transactions...");

    let mut subscription_id: Option<u64> = None;
    let mut tx_count = 0;

    let semaphore = Arc::new(Semaphore::new(5));

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<RpcResponse>(&text) {
                    Ok(response) => {
                        if let Some(error) = response.error {
                            error!("❌ Subscription error: {:?}", error);
                            continue;
                        }

                        if response.id == 1 && response.result.is_some() {
                            subscription_id = response.result.as_ref()
                                .and_then(|v| v.as_u64());
                            
                            info!("✅ Logs subscription confirmed!");
                            info!("   Subscription ID: {:?}", subscription_id);
                            info!("   Waiting for pump.fun events...");
                            continue;
                        }
                       
                        if response.method.as_deref() == Some("logsNotification") {
                            if let Some(params) = response.params {
                                match serde_json::from_value::<LogsNotification>(params) {
                                    Ok(notification) => {

                                        if notification.result.value.err.is_some() {
                                            continue;
                                        }

                                        let signature = notification.result.value.signature.clone();
                                        
                                        let has_pump_event = notification.result.value.logs.iter().any(|log| {
                                            log.contains("Program data:")
                                        });

                                        if !has_pump_event {
                                            continue;
                                        }

                                        tx_count += 1;
                                        
                                        if tx_count == 1 {
                                            info!(" First pump.fun event detected!");
                                        }
                                        
                                        if tx_count % 10 == 0 {
                                            info!("📊 Progress: {} detected", tx_count);
                                        }
                                        
                                        let fetch_signature = signature.clone();
                                        let fetch_rpc_url = format!("https://mainnet.helius-rpc.com/?api-key={}", api_key);
                                        let fetch_sender = tx_sender.clone();
                                        let fetch_tx_count = tx_count;
                                        let permit = semaphore.clone();
                                        
                                        tokio::spawn(async move {
                                            let _permit = permit.acquire().await.unwrap();
                                            
                                            let fetch_rpc = solana_client::rpc_client::RpcClient::new_with_commitment(
                                                fetch_rpc_url,
                                                CommitmentConfig::confirmed(),
                                            );
                                            
                                            tokio::time::sleep(Duration::from_secs(2)).await;
                                            
                                            for attempt in 1..=3 {
                                                match Signature::from_str(&fetch_signature) {
                                                    Ok(sig) => {
                                                        
                                                        let config = solana_client::rpc_config::RpcTransactionConfig {
                                                            encoding: Some(solana_transaction_status::UiTransactionEncoding::JsonParsed),
                                                            commitment: Some(CommitmentConfig::confirmed()),
                                                            max_supported_transaction_version: Some(0), 
                                                        };
                                                        
                                                        match fetch_rpc.get_transaction_with_config(&sig, config) {
                                                            Ok(tx_response) => {
                                                                let raw_tx = RawTransaction {
                                                                    signature: fetch_signature.clone(),
                                                                    slot: tx_response.slot,
                                                                    transaction: tx_response.transaction,
                                                                };

                                                                if let Err(e) = fetch_sender.send(raw_tx) {
                                                                    error!("❌ Failed to send transaction: {}", e);
                                                                } else {
                                                                    info!("✅ TX #{}: {} (attempt {})", 
                                                                        fetch_tx_count, 
                                                                        &fetch_signature[..8], 
                                                                        attempt);
                                                                }
                                                                break;
                                                            }
                                                            Err(e) => {
                                                                if attempt < 3 {
                                                                    debug!("Retry {}/3 for {}...: {}", 
                                                                        attempt, &fetch_signature[..8], e);
                                                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                                                } else {
                                                                    warn!("⚠️ Skipped {}... after 3 attempts", 
                                                                        &fetch_signature[..8]);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!("⚠️ Invalid signature: {}", e);
                                                        break;
                                                    }
                                                }
                                            }
                                            
                                        });
                                    }
                                    Err(e) => {
                                        debug!("Failed to parse notification: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to parse response: {}", e);
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                debug!("🏓 Ping");
                let _ = write.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(frame)) => {
                warn!("⚠️ WebSocket closed: {:?}", frame);
                break;
            }
            Err(e) => {
                error!("❌ WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    warn!("⚠️ WebSocket stream ended");
    Ok(())
}