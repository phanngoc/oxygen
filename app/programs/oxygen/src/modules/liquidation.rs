use anchor_lang::prelude::*;
use std::collections::HashMap;
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;
use crate::modules::collateral::CollateralManager;

/// Module for handling liquidations of unhealthy positions
pub struct LiquidationEngine;

impl LiquidationEngine {
    /// Check if a position can be liquidated
    pub fn can_liquidate_position(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<bool> {
        const LIQUIDATION_THRESHOLD: u64 = 10000; // 1.0 in basis points
        
        // Calculate health factor
        user_position.calculate_health_factor(pool_data)?;
        
        // Can be liquidated if health factor is below threshold
        Ok(user_position.health_factor < LIQUIDATION_THRESHOLD)
    }
    
    /// Calculate liquidation value for a specific debt
    pub fn calculate_liquidation_amount(
        debt_amount: u64,
        debt_pool: &Pool,
        collateral_pool: &Pool,
        collateral_price: u64,
        debt_price: u64
    ) -> Result<u64> {
        // Calculate base collateral value equivalent to debt
        let debt_value = (debt_amount as u128)
            .checked_mul(debt_price as u128)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Apply liquidation bonus
        let collateral_value_with_bonus = debt_value
            .checked_mul(10000 + debt_pool.liquidation_bonus as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Convert to collateral token amount
        let collateral_amount = collateral_value_with_bonus
            .checked_div(collateral_price as u128)
            .ok_or(ErrorCode::MathOverflow)?;
            
        Ok(collateral_amount as u64)
    }
    
    /// Find the optimal debt position to liquidate
    pub fn find_optimal_debt_to_liquidate(
        user_position: &UserPosition,
        max_liquidation_value: u64,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<Option<(usize, u64)>> {
        if user_position.borrows.is_empty() {
            return Ok(None);
        }
        
        let mut best_position: Option<(usize, u64)> = None;
        let mut highest_value: u64 = 0;
        
        // Find the debt position with highest value that's under the max liquidation value
        for (i, borrow) in user_position.borrows.iter().enumerate() {
            if let Some((price, _)) = pool_data.get(&borrow.pool) {
                let value = (borrow.amount_borrowed as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)? as u64;
                
                let amount_to_liquidate = if value > max_liquidation_value {
                    // If the debt is larger than max, liquidate only part of it
                    (max_liquidation_value as u128)
                        .checked_mul(borrow.amount_borrowed as u128)
                        .ok_or(ErrorCode::MathOverflow)?
                        .checked_div(value as u128)
                        .ok_or(ErrorCode::MathOverflow)? as u64
                } else {
                    // Otherwise liquidate the entire position
                    borrow.amount_borrowed
                };
                
                if amount_to_liquidate > 0 && value > highest_value {
                    best_position = Some((i, amount_to_liquidate));
                    highest_value = value;
                }
            }
        }
        
        Ok(best_position)
    }
    
    /// Execute liquidation and update positions
    pub fn execute_liquidation(
        user_position: &mut UserPosition,
        debt_pool: &mut Pool,
        collateral_pool: &mut Pool,
        debt_amount: u64,
        collateral_amount: u64,
        debt_position_idx: usize,
        collateral_position_idx: usize
    ) -> Result<()> {
        // Update user's debt position
        let debt_position = &mut user_position.borrows[debt_position_idx];
        
        debt_position.amount_borrowed = debt_position.amount_borrowed
            .checked_sub(debt_amount)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Remove debt position if fully repaid
        if debt_position.amount_borrowed == 0 {
            user_position.borrows.remove(debt_position_idx);
        }
        
        // Update user's collateral position
        let collateral_position = &mut user_position.collaterals[collateral_position_idx];
        
        collateral_position.amount_deposited = collateral_position.amount_deposited
            .checked_sub(collateral_amount)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Remove collateral position if fully liquidated
        if collateral_position.amount_deposited == 0 {
            user_position.collaterals.remove(collateral_position_idx);
        }
        
        // Update pool totals
        debt_pool.total_borrows = debt_pool.total_borrows
            .checked_sub(debt_amount)
            .ok_or(ErrorCode::MathOverflow)?;
            
        collateral_pool.total_deposits = collateral_pool.total_deposits
            .checked_sub(collateral_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        Ok(())
    }
    
    /// Calculate the max amount that can be liquidated at once
    pub fn calculate_max_liquidation_amount(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<u64> {
        let total_borrow_value = CollateralManager::calculate_total_borrow_value(
            user_position, 
            pool_data
        )?;
        
        // In Oxygen protocol, we use a close factor of 50%
        // This means a liquidator can close up to 50% of the borrower's debt in a single tx
        const CLOSE_FACTOR: u64 = 5000; // 50% in basis points
        
        let max_liquidation_value = (total_borrow_value as u128)
            .checked_mul(CLOSE_FACTOR as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        Ok(max_liquidation_value)
    }
}