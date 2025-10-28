use sqlx::PgPool;
use tokio::time::{interval, Duration};
use tracing::{info, error};
use crate::processor::state::TokenStateMap;

pub async fn start_state_backup(pool: PgPool, state_map: TokenStateMap) {
    let mut interval = interval(Duration::from_secs(60));
    
    info!("💾 Starting state backup task (60s interval)");
    
    loop {
        interval.tick().await;
        
        let states = {
            let map = state_map.read().await;
            map.values().cloned().collect::<Vec<_>>()
        };
        
        if states.is_empty() {
            continue;
        }
        
        info!("💾 Backing up {} token states to database...", states.len());
        
        let mut updated = 0;
        let mut failed = 0;
        
        for state in states {
            let result = sqlx::query(
                "UPDATE tokens 
                 SET market_cap_sol = $1, bonding_curve_progress = $2, last_updated = NOW()
                 WHERE mint = $3"
            )
            .bind(state.market_cap_sol)
            .bind(state.bonding_curve_progress)
            .bind(&state.mint)
            .execute(&pool)
            .await;
            
            match result {
                Ok(_) => updated += 1,
                Err(e) => {
                    error!("Failed to update token {}: {}", state.mint, e);
                    failed += 1;
                }
            }
        }
        
        info!("💾 State backup complete: {} updated, {} failed", updated, failed);
    }
}