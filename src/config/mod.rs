use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
  pub database_url: String,
  pub redis_url: String,
  pub helius_api_key: String,
  pub api_port: u16,
}

impl Config {
  pub fn from_env() -> Result<Self> {
    dotenv::dotenv().ok();

    Ok(Self {
      database_url: env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env"),

      redis_url: env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string()),

      helius_api_key: env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set in .env"),
        
      api_port: env::var("API_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("API_PORT must be a valid number"),
    })
  }
}