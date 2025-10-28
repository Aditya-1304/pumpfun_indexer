use redis::aio::ConnectionManager;
use redis::{Client, AsyncCommands, RedisResult};
use anyhow::{Result, Context};
use serde_json;
use tracing::{info, error, warn};

#[derive(Clone)]
pub struct RedisClient {
    pub connection: ConnectionManager,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        info!("  Connecting to Redis: {}", mask_redis_url(redis_url));

        let client = Client::open(redis_url)
            .context("Failed to create Redis Client")?;

        let connection = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        info!("  Redis connected successfully");

        Ok(Self { connection })
    }

    pub async fn publish<T: serde::Serialize>(
        &mut self,
        channel: &str,
        message: &T,
    ) -> Result<()> {
        let json = serde_json::to_string(message)
            .context("Failed to serialize message")?;

        match self.connection.publish::<_, _, ()>(channel, json.clone()).await {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("  Redis publish error: {}", e);
                
                if e.is_connection_dropped() || e.is_io_error() {
                    warn!("  Redis connection lost, attempting reconnect...");
                
                }
                
                Err(e.into())
            }
        }
    }

    pub async fn set<T: serde::Serialize>(
        &mut self,
        key: &str,
        value: &T,
        expiry_seconds: Option<usize>,
    ) -> Result<()> {
        let json = serde_json::to_string(value)
            .context("Failed to serialize value")?;
        
        if let Some(seconds) = expiry_seconds {
            self.connection
                .set_ex::<_, _, ()>(key, json, seconds as u64)
                .await
                .context("Failed to set key with expiry")?;
        } else {
            self.connection
                .set::<_, _, ()>(key, json)
                .await
                .context("Failed to set key")?;
        }
        
        Ok(())
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>> {
        let result: RedisResult<String> = self.connection.get(key).await;
        
        match result {
            Ok(json) => {
                let value = serde_json::from_str(&json)
                    .context("Failed to deserialize value")?;
                Ok(Some(value))
            }
            Err(e) if e.kind() == redis::ErrorKind::TypeError => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn delete(&mut self, key: &str) -> Result<()> {
        self.connection
            .del::<_, ()>(key)
            .await
            .context("Failed to delete key")?;
        
        Ok(())
    }
    
    pub async fn increment(&mut self, key: &str) -> Result<i64> {
        let value = self.connection
            .incr(key, 1)
            .await
            .context("Failed to increment counter")?;
        
        Ok(value)
    }

    pub async fn ping(&mut self) -> Result<()> {
        redis::cmd("PING")
            .query_async::<String>(&mut self.connection)
            .await
            .context("Redis PING failed")?;
        Ok(())
    }
}

pub async fn create_redis_client(redis_url: &str) -> Result<RedisClient> {
    RedisClient::new(redis_url).await
}

fn mask_redis_url(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            masked.replace_range(colon_pos + 1..at_pos, "****");
            return masked;
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mask_redis_url() {
        let url = "redis://user:password@localhost:6379/0";
        let masked = mask_redis_url(url);
        assert_eq!(masked, "redis://user:****@localhost:6379/0");
    }
}