pub mod model;

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
  info!("Connecting to database...");

  let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(std::time::Duration::from_secs(10))
    .connect(database_url)
    .await?;

  info!("Database connection established");
  Ok(pool)
}

pub async fn test_connection(pool: &PgPool) -> Result<()> {
  let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tokens")
    .fetch_one(pool)
    .await?;
  info!("Database has {} tokens", row.0);
  Ok(())
}