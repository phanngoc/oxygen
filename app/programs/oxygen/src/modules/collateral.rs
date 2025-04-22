use anchor_lang::prelude::*;
use std::collections::HashMap;
use crate::state::{Pool, UserPosition, CollateralPosition};
use crate::errors::OxygenError;

/// Module for managing cross-collateralization and collateral calculations
pub struct CollateralManager;

impl CollateralManager {
    /// Calculate total collateral value across all user positions
    pub fn calculate_total_collateral_value(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)> // Map of (pool_address => (price, liquidation_threshold))
    ) -> Result<u128> {
        let mut total_collateral_value = 0u128;
        
        for collateral in &user_position.collaterals {
            if !collateral.is_collateral {
                continue; // Skip non-collateral deposits
            }
            
            // Get price and liquidation threshold for this asset
            if let Some((price, _)) = pool_data.get(&collateral.pool) {
                // Calculate collateral value: amount * price
                let value = (collateral.amount_deposited as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                // Add to total
                total_collateral_value = total_collateral_value
                    .checked_add(value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        Ok(total_collateral_value)
    }
    
    /// Calculate weighted collateral value (applying liquidation thresholds)
    pub fn calculate_weighted_collateral_value(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)> // Map of (pool_address => (price, liquidation_threshold))
    ) -> Result<u128> {
        let mut total_weighted_value = 0u128;
        
        for collateral in &user_position.collaterals {
            if !collateral.is_collateral {
                continue; // Skip non-collateral deposits
            }
            
            // Get price and liquidation threshold for this asset
            if let Some((price, liquidation_threshold)) = pool_data.get(&collateral.pool) {
                // Calculate base value: amount * price
                let value = (collateral.amount_deposited as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                // Apply liquidation threshold to get weighted value
                let weighted_value = value
                    .checked_mul(*liquidation_threshold as u128)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(10000) // Assuming liquidation threshold is in basis points
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                // Add to total
                total_weighted_value = total_weighted_value
                    .checked_add(weighted_value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        Ok(total_weighted_value)
    }
    
    /// Calculate total borrowed value across all user borrows
    pub fn calculate_total_borrow_value(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)> // Map of (pool_address => (price, liquidation_threshold))
    ) -> Result<u128> {
        let mut total_borrow_value = 0u128;
        
        for borrow in &user_position.borrows {
            // Get price for this asset
            if let Some((price, _)) = pool_data.get(&borrow.pool) {
                // Calculate borrow value: amount * price
                let value = (borrow.amount_borrowed as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                // Add to total
                total_borrow_value = total_borrow_value
                    .checked_add(value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        Ok(total_borrow_value)
    }
    
    /// Check if a user can borrow more based on their collateral
    pub fn can_borrow_more(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)>,
        additional_borrow_value: u128,
        min_health_factor: u64
    ) -> Result<bool> {
        let weighted_collateral_value = Self::calculate_weighted_collateral_value(user_position, pool_data)?;
        let current_borrow_value = Self::calculate_total_borrow_value(user_position, pool_data)?;
        
        // Calculate new hypothetical borrow value
        let new_total_borrow = current_borrow_value
            .checked_add(additional_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;
            
        if new_total_borrow == 0 {
            return Ok(true); // No borrows at all
        }
        
        // Calculate new hypothetical health factor
        let new_health_factor = weighted_collateral_value
            .checked_mul(10000) // Scale for precision
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(new_total_borrow)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        // Check if health factor would remain above minimum
        Ok(new_health_factor >= min_health_factor)
    }
    
    /// Find the maximum borrowable amount for a specific asset
    pub fn find_max_borrowable_amount(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)>,
        borrow_pool: &Pubkey,
        min_health_factor: u64
    ) -> Result<u64> {
        // Get asset price
        let asset_price = if let Some((price, _)) = pool_data.get(borrow_pool) {
            *price as u128
        } else {
            return Err(OxygenError::InvalidParameter.into());
        };
        
        let weighted_collateral_value = Self::calculate_weighted_collateral_value(user_position, pool_data)?;
        let current_borrow_value = Self::calculate_total_borrow_value(user_position, pool_data)?;
        
        if weighted_collateral_value == 0 {
            return Ok(0); // No collateral, can't borrow
        }
        
        // Calculate max additional borrow value while maintaining health factor
        // Formula: max_borrow_value = weighted_collateral_value / min_health_factor - current_borrow_value
        let scaled_collateral = weighted_collateral_value
            .checked_mul(10000) // Scale for precision
            .ok_or(ErrorCode::MathOverflow)?;
            
        let max_total_borrow = scaled_collateral
            .checked_div(min_health_factor as u128)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // If already borrowed more than allowed, can't borrow more
        if current_borrow_value >= max_total_borrow {
            return Ok(0);
        }
        
        let max_additional_value = max_total_borrow
            .checked_sub(current_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Convert value to token amount using asset price
        let max_borrowable_amount = max_additional_value
            .checked_div(asset_price)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        Ok(max_borrowable_amount)
    }
    
    /// Check if a position is eligible for liquidation
    pub fn is_liquidatable(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)>,
        liquidation_threshold: u64
    ) -> Result<bool> {
        let weighted_collateral_value = Self::calculate_weighted_collateral_value(user_position, pool_data)?;
        let borrow_value = Self::calculate_total_borrow_value(user_position, pool_data)?;
        
        if borrow_value == 0 {
            return Ok(false); // No borrows, can't liquidate
        }
        
        // Calculate health factor
        let health_factor = weighted_collateral_value
            .checked_mul(10000) // Scale for precision
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(borrow_value)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        // Position is liquidatable if health factor is below threshold
        Ok(health_factor < liquidation_threshold)
    }
}