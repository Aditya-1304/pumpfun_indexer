
pub fn calculate_price_sol(
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
) -> f64 {
    if virtual_token_reserves == 0 {
        return 0.0;
    }
    
    let sol = virtual_sol_reserves as f64 / 1_000_000_000.0;
    let tokens = virtual_token_reserves as f64 / 1_000_000.0;
    
    sol / tokens
}

pub fn calculate_market_cap_sol(
    price_sol: f64,
    total_supply: u64,
) -> f64 {
    let supply = total_supply as f64 / 1_000_000.0;
    price_sol * supply
}

pub fn calculate_bonding_curve_progress(virtual_sol_reserves: u64) -> f64 {
    const TARGET_SOL: f64 = 85.0; // SOL needed to complete curve
    let sol_in_curve = virtual_sol_reserves as f64 / 1_000_000_000.0;
    ((sol_in_curve / TARGET_SOL) * 100.0).min(100.0).max(0.0)
}

pub fn calculate_price_impact(
    trade_sol_amount: u64,
    virtual_sol_reserves: u64,
) -> f64 {
    if virtual_sol_reserves == 0 {
        return 0.0;
    }
    
    let trade_sol = trade_sol_amount as f64 / 1_000_000_000.0;
    let reserves_sol = virtual_sol_reserves as f64 / 1_000_000_000.0;
    
    (trade_sol / reserves_sol) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_price_calculation() {
        let price = calculate_price_sol(10_000_000_000, 1_000_000_000_000);
        assert!((price - 0.00001).abs() < 0.000001);
    }
    
    #[test]
    fn test_bonding_curve_progress() {
        let progress = calculate_bonding_curve_progress(42_500_000_000);
        assert!((progress - 50.0).abs() < 0.1);
    }
}