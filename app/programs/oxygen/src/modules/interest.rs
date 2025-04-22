use anchor_lang::prelude::*;
use crate::state::Pool;
use crate::errors::OxygenError;

/// Module for managing interest rate models and calculations
pub struct InterestRateModel;

impl InterestRateModel {
    /// Calculate borrow interest rate based on pool utilization
    pub fn calculate_borrow_rate(
        utilization_rate: u64,
        optimal_utilization: u64,
        base_rate: u64,
        slope1: u64,
        slope2: u64
    ) -> Result<u64> {
        let borrow_rate = if utilization_rate <= optimal_utilization {
            // Below optimal: Use slope1
            base_rate.checked_add(
                utilization_rate
                    .checked_mul(slope1)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(optimal_utilization)
                    .ok_or(ErrorCode::MathOverflow)?
            ).ok_or(ErrorCode::MathOverflow)?
        } else {
            // Above optimal: Use slope2 and add the first part
            let base_part = base_rate.checked_add(slope1).ok_or(ErrorCode::MathOverflow)?;
            
            let excess_utilization = utilization_rate
                .checked_sub(optimal_utilization)
                .ok_or(ErrorCode::MathOverflow)?;
            
            let max_excess = 10000u64.checked_sub(optimal_utilization).ok_or(ErrorCode::MathOverflow)?;
            
            let excess_rate = excess_utilization
                .checked_mul(slope2)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(max_excess)
                .ok_or(ErrorCode::MathOverflow)?;
            
            base_part.checked_add(excess_rate).ok_or(ErrorCode::MathOverflow)?
        };
        
        Ok(borrow_rate)
    }
    
    /// Calculate supply interest rate based on borrow rate and utilization
    pub fn calculate_supply_rate(
        borrow_rate: u64,
        utilization_rate: u64,
        reserve_factor: u64
    ) -> Result<u64> {
        // Supply rate = borrow rate * utilization rate * (1 - reserve factor)
        let borrow_part = (borrow_rate as u128)
            .checked_mul(utilization_rate as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000)
            .ok_or(ErrorCode::MathOverflow)?;
            
        let reserve_factor_scaled = (reserve_factor as u128)
            .checked_mul(borrow_part)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000)
            .ok_or(ErrorCode::MathOverflow)?;
            
        let supply_rate = borrow_part
            .checked_sub(reserve_factor_scaled)
            .ok_or(ErrorCode::MathOverflow)?;
            
        Ok(supply_rate as u64)
    }
    
    /// Update cumulative interest rate of a pool
    pub fn update_cumulative_rate(
        pool: &mut Pool,
        current_timestamp: i64
    ) -> Result<()> {
        if pool.total_deposits == 0 || pool.last_updated == current_timestamp {
            return Ok(());
        }
        
        // Calculate utilization rate
        let utilization_rate = (pool.total_borrows as u128)
            .checked_mul(10000)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(pool.total_deposits as u128)
            .unwrap_or(0) as u64;
            
        // Calculate borrow rate using standard parameters
        // These parameters could be customized per pool in a full implementation
        let base_rate = 200; // 2% base rate
        let slope1 = 800;    // 8% slope up to optimal utilization
        let slope2 = 3000;   // 30% slope beyond optimal utilization
        
        let borrow_rate = Self::calculate_borrow_rate(
            utilization_rate,
            pool.optimal_utilization,
            base_rate,
            slope1,
            slope2
        )?;
        
        // Calculate time elapsed since last update (in seconds)
        let time_elapsed = (current_timestamp - pool.last_updated) as u128;
        
        // Update cumulative borrow rate
        // Formula: previous_rate * (1 + borrow_rate * time_elapsed / SECONDS_PER_YEAR)
        const SECONDS_PER_YEAR: u128 = 31536000; // 365 * 24 * 60 * 60
        
        let borrow_rate_factor = (borrow_rate as u128)
            .checked_mul(time_elapsed)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(SECONDS_PER_YEAR)
            .ok_or(ErrorCode::MathOverflow)?;
            
        let borrow_rate_multipler = 10000u128
            .checked_add(borrow_rate_factor)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Apply the compound interest
        pool.cumulative_borrow_rate = (pool.cumulative_borrow_rate)
            .checked_mul(borrow_rate_multipler)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000)
            .ok_or(ErrorCode::MathOverflow)?;
            
        pool.last_updated = current_timestamp;
        
        Ok(())
    }
}