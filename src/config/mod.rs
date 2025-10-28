use anyhow::{Result, Context};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub helius_api_key: String,
    pub redis_url: String,
    pub api_port: u16,
    pub coingecko_api_key: Option<String>, // 🔥 NEW: Optional API key
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            
            helius_api_key: env::var("HELIUS_API_KEY")
                .context("HELIUS_API_KEY must be set")?,
            
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            
            api_port: env::var("API_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("API_PORT must be a valid number")?,
            
            // 🔥 NEW: Load CoinGecko API key (optional)
            coingecko_api_key: env::var("COINGECKO_API_KEY").ok(),
        })
    }
}