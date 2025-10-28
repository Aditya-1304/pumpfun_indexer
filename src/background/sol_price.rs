use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, error, warn};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PythResponse {
    #[serde(default)]
    data: Vec<PriceData>,
}

#[derive(Debug, Deserialize)]
struct PriceData {
    id: String,
    price: PriceInfo,
}

#[derive(Debug, Deserialize)]
struct PriceInfo {
    price: String,
    #[serde(rename = "conf")]
    confidence: String,
    expo: i32,
    #[serde(rename = "publish_time")]
    publish_time: i64,
}

async fn fetch_sol_price_pyth() -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let price_feed_id = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
    
    let url = format!(
        "https://hermes.pyth.network/v2/updates/price/latest?ids[]={}&encoding=hex",
        price_feed_id
    );
    
    let response = client
        .get(&url)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Pyth API error: {}", response.status()).into());
    }
    
    #[derive(Deserialize)]
    struct HermesResponse {
        parsed: Vec<ParsedData>,
    }
    
    #[derive(Deserialize)]
    struct ParsedData {
        id: String,
        price: ParsedPrice,
    }
    
    #[derive(Deserialize)]
    struct ParsedPrice {
        price: String,
        expo: i32,
    }
    
    let data: HermesResponse = response.json().await?;
    
    if let Some(parsed) = data.parsed.first() {
        let price_raw: i64 = parsed.price.price.parse()?;
        let expo = parsed.price.expo;
        let price = (price_raw as f64) * 10_f64.powi(expo);
        return Ok(price);
    }
    
    Err("No price data in Pyth response".into())
}

async fn fetch_sol_price_coingecko(api_key: Option<String>) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";
    
    let mut request = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0")
        .header("Accept", "application/json");
    
    if let Some(key) = api_key {
        request = request.header("x-cg-demo-api-key", key);
    }
    
    let response = request.send().await?;
    
    #[derive(Deserialize)]
    struct CoinGeckoResponse {
        solana: SolanaPrice,
    }
    
    #[derive(Deserialize)]
    struct SolanaPrice {
        usd: f64,
    }
    
    let data: CoinGeckoResponse = response.json().await?;
    Ok(data.solana.usd)
}

async fn fetch_sol_price(api_key: Option<String>) -> f64 {
    match fetch_sol_price_pyth().await {
        Ok(price) => {
            info!("💰 Fetched from Pyth: ${:.2}", price);
            return price;
        }
        Err(e) => {
            warn!("⚠️ Pyth failed: {}", e);
        }
    }
    
    match fetch_sol_price_coingecko(api_key).await {
        Ok(price) => {
            info!("💰 Fetched from CoinGecko: ${:.2}", price);
            return price;
        }
        Err(e) => {
            warn!("⚠️ CoinGecko failed: {}", e);
        }
    }
    
    warn!("⚠️ All price sources failed, using fallback: $150.00");
    150.0
}

pub async fn start_sol_price_updater(
    sol_price: Arc<RwLock<f64>>,
    api_key: Option<String>,
) {
    let mut interval = interval(Duration::from_secs(15));
    
    info!("💰 Starting SOL price updater (Pyth + CoinGecko fallback, 15s interval)");
    
    let initial_price = fetch_sol_price(api_key.clone()).await;
    *sol_price.write().await = initial_price;
    info!("💰 Initial SOL price: ${:.2}", initial_price);
    
    loop {
        interval.tick().await;
        
        let price = fetch_sol_price(api_key.clone()).await;
        let old_price = *sol_price.read().await;
        *sol_price.write().await = price;
        
        let change = ((price - old_price) / old_price) * 100.0;
        
        if change.abs() > 0.5 {
            info!("💰 SOL price updated: ${:.2} ({:+.2}%)", price, change);
        }
    }
}