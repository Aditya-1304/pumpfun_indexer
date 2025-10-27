use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Metrics {
    pub tokens_created: Arc<AtomicU64>,
    pub trades_processed: Arc<AtomicU64>,
    pub tokens_graduated: Arc<AtomicU64>,
    pub redis_publish_errors: Arc<AtomicU64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            tokens_created: Arc::new(AtomicU64::new(0)),
            trades_processed: Arc::new(AtomicU64::new(0)),
            tokens_graduated: Arc::new(AtomicU64::new(0)),
            redis_publish_errors: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn increment_tokens_created(&self) {
        self.tokens_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_trades_processed(&self) {
        self.trades_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_tokens_graduated(&self) {
        self.tokens_graduated.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_redis_errors(&self) {
        self.redis_publish_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            tokens_created: self.tokens_created.load(Ordering::Relaxed),
            trades_processed: self.trades_processed.load(Ordering::Relaxed),
            tokens_graduated: self.tokens_graduated.load(Ordering::Relaxed),
            redis_publish_errors: self.redis_publish_errors.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub tokens_created: u64,
    pub trades_processed: u64,
    pub tokens_graduated: u64,
    pub redis_publish_errors: u64,
}