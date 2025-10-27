use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};


#[derive(Debug, Clone)]
pub struct TokenState {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub creator: String,
    
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    
    
    pub current_price_sol: f64,
    pub market_cap_sol: f64,
    pub market_cap_usd: f64,
    pub bonding_curve_progress: f64,
    
    
    pub total_supply: u64,
    pub complete: bool,
    pub last_updated: DateTime<Utc>,
}


pub type TokenStateMap = Arc<RwLock<HashMap<String, TokenState>>>;

pub fn create_state_map() -> TokenStateMap {
    Arc::new(RwLock::new(HashMap::new()))
}

pub async fn init_token_state(
    state_map: &TokenStateMap,
    mint: String,
    name: String,
    symbol: String,
    creator: String,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_token_reserves: u64,
    total_supply: u64,
    sol_price_usd: f64,
) {
    let mut map = state_map.write().await;
    
    let price_sol = if virtual_token_reserves > 0 {
        (virtual_sol_reserves as f64 / 1_000_000_000.0) / 
        (virtual_token_reserves as f64 / 1_000_000.0)
    } else {
        0.0
    };
    
    let market_cap_sol = price_sol * (total_supply as f64 / 1_000_000.0);
    let market_cap_usd = market_cap_sol * sol_price_usd;
    

    const TARGET_SOL: f64 = 85.0;
    let sol_in_curve = virtual_sol_reserves as f64 / 1_000_000_000.0;
    let progress = ((sol_in_curve / TARGET_SOL) * 100.0).min(100.0).max(0.0);
    
    let token_state = TokenState {
        mint: mint.clone(),
        name,
        symbol,
        creator,
        virtual_sol_reserves,
        virtual_token_reserves,
        real_sol_reserves: 0,
        real_token_reserves,
        current_price_sol: price_sol,
        market_cap_sol,
        market_cap_usd,
        bonding_curve_progress: progress,
        total_supply,
        complete: false,
        last_updated: Utc::now(),
    };
    
    map.insert(mint, token_state);
}

pub async fn update_token_state(
    state_map: &TokenStateMap,
    mint: &str,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_sol_reserves: u64,
    real_token_reserves: u64,
    sol_price_usd: f64,
) -> Option<TokenState> {
    let mut map = state_map.write().await;
    
    if let Some(state) = map.get_mut(mint) {
        state.virtual_sol_reserves = virtual_sol_reserves;
        state.virtual_token_reserves = virtual_token_reserves;
        state.real_sol_reserves = real_sol_reserves;
        state.real_token_reserves = real_token_reserves;
        

        state.current_price_sol = if virtual_token_reserves > 0 {
            (virtual_sol_reserves as f64 / 1_000_000_000.0) / 
            (virtual_token_reserves as f64 / 1_000_000.0)
        } else {
            0.0
        };
        
        state.market_cap_sol = state.current_price_sol * (state.total_supply as f64 / 1_000_000.0);
        state.market_cap_usd = state.market_cap_sol * sol_price_usd;
        

        const TARGET_SOL: f64 = 85.0;
        let sol_in_curve = virtual_sol_reserves as f64 / 1_000_000_000.0;
        state.bonding_curve_progress = ((sol_in_curve / TARGET_SOL) * 100.0).min(100.0).max(0.0);
        
        state.last_updated = Utc::now();
        
        Some(state.clone())
    } else {
        None
    }
}

pub async fn mark_token_complete(state_map: &TokenStateMap, mint: &str) {
    let mut map = state_map.write().await;
    if let Some(state) = map.get_mut(mint) {
        state.complete = true;
        state.bonding_curve_progress = 100.0;
        state.last_updated = Utc::now();
    }
}

pub async fn get_token_state(state_map: &TokenStateMap, mint: &str) -> Option<TokenState> {
    let map = state_map.read().await;
    map.get(mint).cloned()
}

pub async fn get_all_tokens(state_map: &TokenStateMap) -> Vec<TokenState> {
    let map = state_map.read().await;
    map.values().cloned().collect()
}