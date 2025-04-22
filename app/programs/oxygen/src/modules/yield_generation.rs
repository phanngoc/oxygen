use anchor_lang::prelude::*;
use crate::state::{Pool, UserPosition, CollateralPosition};
use crate::errors::OxygenError;

/// Module for managing yield generation and distribution
pub struct YieldGenerator;

impl YieldGenerator {
    /// Calculate accrued yield for a user's deposit in a specific pool
    pub fn calculate_accrued_yield(
        pool: &Pool,
        collateral_position: &CollateralPosition,
    ) -> Result<u64> {
        // If no deposits in pool, no yield
        if pool.total_deposits == 0 {
            return Ok(0);
        }
        
        // Calculate current token equivalent of scaled amount
        let current_token_value = (collateral_position.amount_scaled as u128)
            .checked_mul(pool.total_deposits as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000) // Scale factor for precision
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        // Calculate yield as the difference between current value and original deposit
        let yield_amount = current_token_value
            .checked_sub(collateral_position.amount_deposited)
            .unwrap_or(0);
            
        Ok(yield_amount)
    }
    
    /// Calculate APY for a specific pool
    pub fn calculate_pool_apy(pool: &Pool) -> Result<u64> {
        // If no deposits or no borrows, no yield
        if pool.total_deposits == 0 || pool.total_borrows == 0 {
            return Ok(0);
        }
        
        // Calculate utilization rate
        let utilization_rate = (pool.total_borrows as u128)
            .checked_mul(10000) // Basis points
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(pool.total_deposits as u128)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        // Calculate borrow rate based on utilization (simplified model)
        // In a real implementation, this would use a more sophisticated model
        let borrow_rate = if utilization_rate <= pool.optimal_utilization {
            // Below optimal utilization: linear increase
            (utilization_rate as u128)
                .checked_mul(500) // 0-5% range
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(pool.optimal_utilization as u128)
                .ok_or(ErrorCode::MathOverflow)? as u64
        } else {
            // Above optimal utilization: steeper increase
            let base_rate = 500; // 5% at optimal utilization
            
            let excess_utilization = utilization_rate
                .checked_sub(pool.optimal_utilization)
                .ok_or(ErrorCode::MathOverflow)?;
                
            let max_excess = 10000u64
                .checked_sub(pool.optimal_utilization)
                .ok_or(ErrorCode::MathOverflow)?;
                
            let additional_rate = (excess_utilization as u128)
                .checked_mul(1500) // Additional 0-15% for excess utilization
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(max_excess as u128)
                .ok_or(ErrorCode::MathOverflow)? as u64;
                
            base_rate
                .checked_add(additional_rate)
                .ok_or(ErrorCode::MathOverflow)?
        };
        
        // Calculate supply APY based on borrow rate and utilization
        let supply_apy = (borrow_rate as u128)
            .checked_mul(utilization_rate as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000) // Adjust for basis points
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        Ok(supply_apy)
    }
    
    /// Calculate total yield earned across all pools for a user
    pub fn calculate_total_yield_earned(
        user_position: &UserPosition,
        pools: &[(&Pubkey, &Pool)]
    ) -> Result<u64> {
        let mut total_yield = 0u64;
        
        for collateral in &user_position.collaterals {
            // Find corresponding pool for this collateral
            for (pool_pubkey, pool) in pools {
                if &collateral.pool == *pool_pubkey {
                    let yield_amount = Self::calculate_accrued_yield(
                        pool,
                        collateral
                    )?;
                    
                    total_yield = total_yield
                        .checked_add(yield_amount)
                        .ok_or(ErrorCode::MathOverflow)?;
                        
                    break;
                }
            }
        }
        
        Ok(total_yield)
    }
    
    /// Claim yield from a specific pool
    pub fn claim_yield(
        pool: &mut Pool,
        collateral_position: &mut CollateralPosition,
        reinvest: bool
    ) -> Result<u64> {
        // Calculate accrued yield
        let yield_amount = Self::calculate_accrued_yield(pool, collateral_position)?;
        
        if yield_amount == 0 {
            return Ok(0);
        }
        
        if reinvest {
            // If reinvesting, update deposit amount to include yield
            collateral_position.amount_deposited = collateral_position.amount_deposited
                .checked_add(yield_amount)
                .ok_or(ErrorCode::MathOverflow)?;
        } else {
            // If not reinvesting, update deposit amount to match current value
            let current_token_value = (collateral_position.amount_scaled as u128)
                .checked_mul(pool.total_deposits as u128)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(10000) // Scale factor
                .ok_or(ErrorCode::MathOverflow)? as u64;
                
            collateral_position.amount_deposited = current_token_value;
        }
        
        Ok(yield_amount)
    }
    
    /// Update index when interest accrues to accurately track yield
    pub fn update_index(
        pool: &mut Pool,
        current_timestamp: i64
    ) -> Result<()> {
        // Skip if no time has passed or no deposits
        if pool.last_updated == current_timestamp || pool.total_deposits == 0 {
            return Ok(());
        }
        
        // Calculate utilization rate
        let utilization_rate = (pool.total_borrows as u128)
            .checked_mul(10000)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(pool.total_deposits as u128)
            .unwrap_or(0) as u64;
            
        // Calculate borrow rate (simplified)
        let borrow_rate = if utilization_rate <= pool.optimal_utilization {
            (utilization_rate as u128)
                .checked_mul(500)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(pool.optimal_utilization as u128)
                .ok_or(ErrorCode::MathOverflow)? as u64
        } else {
            let base_rate = 500;
            
            let excess_utilization = utilization_rate
                .checked_sub(pool.optimal_utilization)
                .ok_or(ErrorCode::MathOverflow)?;
                
            let max_excess = 10000u64
                .checked_sub(pool.optimal_utilization)
                .ok_or(ErrorCode::MathOverflow)?;
                
            let additional_rate = (excess_utilization as u128)
                .checked_mul(1500)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(max_excess as u128)
                .ok_or(ErrorCode::MathOverflow)? as u64;
                
            base_rate
                .checked_add(additional_rate)
                .ok_or(ErrorCode::MathOverflow)?
        };
        
        // Calculate time elapsed in seconds
        let time_elapsed = (current_timestamp - pool.last_updated) as u128;
        
        // Calculate interest: rate * time / year
        const SECONDS_PER_YEAR: u128 = 31536000; // 365 * 24 * 60 * 60
        
        let interest_factor = (borrow_rate as u128)
            .checked_mul(time_elapsed)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(SECONDS_PER_YEAR)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Update pool index for yield tracking
        pool.cumulative_borrow_rate = pool.cumulative_borrow_rate
            .checked_add(interest_factor)
            .ok_or(ErrorCode::MathOverflow)?;
            
        pool.last_updated = current_timestamp;
        
        Ok(())
    }
}