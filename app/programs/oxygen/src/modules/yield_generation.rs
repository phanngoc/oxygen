use anchor_lang::prelude::*;
use std::collections::HashMap;
use crate::state::{Pool, UserPosition, CollateralPosition};
use crate::errors::OxygenError;
use crate::modules::wallet_integration::WalletIntegration;

/// Module for handling yield generation and distribution
pub struct YieldModule;

impl YieldModule {
    /// Calculate accrued yield for a user's lending position
    pub fn calculate_accrued_yield(
        pool: &Pool,
        collateral_position: &CollateralPosition,
        current_timestamp: i64
    ) -> Result<u64> {
        // Skip if position isn't being used for lending
        if !collateral_position.is_lending {
            return Ok(0);
        }

        // No yield if no cumulative lending rate or just deposited
        if pool.cumulative_lending_rate == 0 || pool.last_updated == 0 {
            return Ok(0);
        }
        
        // Calculate the ratio of current lending rate to the rate when the deposit was made
        // This gives us the growth factor of the deposit
        let principal_value = collateral_position.amount_deposited;
        
        // Calculate accrued value using the ratio of scaled amount to current exchange rate
        let current_value = (collateral_position.amount_scaled as u128)
            .checked_mul(pool.cumulative_lending_rate)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(1_000_000_000_000) // Scale back from 10^12 precision
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        // Accrued yield is the difference between current value and principal
        let accrued_yield = if current_value > principal_value {
            current_value.checked_sub(principal_value).unwrap_or(0)
        } else {
            0 // Should never happen unless there's a numerical issue
        };
        
        Ok(accrued_yield)
    }
    
    /// Claim accrued yield for a user's lending position
    /// Non-custodial: requires user signature to claim their own yield
    pub fn claim_yield<'a>(
        pool: &mut Account<'a, Pool>,
        user_position: &mut Account<'a, UserPosition>,
        pool_key: &Pubkey,
        current_timestamp: i64,
        user: &Signer<'a>,
    ) -> Result<u64> {
        // First validate non-custodial requirements
        require!(pool.immutable, OxygenError::PoolIsUpgradable);
        require!(pool.admin_less, OxygenError::AdminOperationsNotSupported);
        
        // Non-custodial security: ensure only the owner can claim their yield
        WalletIntegration::validate_owner_signed(&user_position.owner, user)?;
        
        let mut total_accrued_yield = 0u64;
        let mut collateral_index = None;
        
        // Find the collateral position for this pool
        for (i, collateral) in user_position.collaterals.iter().enumerate() {
            if collateral.pool == *pool_key && collateral.is_lending {
                // Calculate accrued yield
                let accrued_yield = Self::calculate_accrued_yield(
                    pool,
                    collateral,
                    current_timestamp
                )?;
                
                total_accrued_yield = accrued_yield;
                collateral_index = Some(i);
                break;
            }
        }
        
        // Ensure we found the collateral position
        if collateral_index.is_none() || total_accrued_yield == 0 {
            // No yield to claim or position not found
            return Ok(0);
        }
        
        // Verify the pool has enough liquidity to pay the yield
        // This should never be a problem unless the protocol is insolvent
        let available_liquidity = pool.total_deposits
            .checked_sub(pool.total_borrows)
            .ok_or(OxygenError::InsufficientLiquidity)?;
            
        require!(
            available_liquidity >= total_accrued_yield,
            OxygenError::InsufficientLiquidity
        );
        
        // Update the collateral position to reflect claimed yield
        let index = collateral_index.unwrap();
        let collateral = &mut user_position.collaterals[index];
        
        // When claiming yield, we need to update the scaled amount to match the current rate
        // This effectively resets the yield calculation
        let new_scaled_amount = (collateral.amount_deposited as u128)
            .checked_mul(1_000_000_000_000) // 10^12 precision
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(pool.cumulative_lending_rate)
            .ok_or(ErrorCode::MathOverflow)?;
            
        collateral.amount_scaled = new_scaled_amount;
        
        // In a full implementation, we would now transfer the yield to the user's wallet
        // Non-custodial: we transfer directly to the user's wallet, not to protocol-controlled accounts
        
        // Return the amount of yield claimed
        Ok(total_accrued_yield)
    }
    
    /// Update lending yields for all deposits in a pool
    pub fn update_pool_yields<'a>(
        pool: &mut Account<'a, Pool>,
        current_timestamp: i64
    ) -> Result<()> {
        // Verify the pool is non-custodial and immutable
        require!(pool.immutable, OxygenError::PoolIsUpgradable);
        require!(pool.admin_less, OxygenError::AdminOperationsNotSupported);
        
        if pool.last_updated == 0 || pool.last_updated == current_timestamp {
            return Ok(());
        }
        
        // Calculate time elapsed since last update
        let time_elapsed = (current_timestamp - pool.last_updated) as u128;
        if time_elapsed == 0 {
            return Ok(());
        }
        
        // Calculate the lending APY based on pool utilization
        let utilization_rate = if pool.available_lending_supply > 0 {
            (pool.total_borrows as u128)
                .checked_mul(10000)
                .unwrap_or(0) / (pool.available_lending_supply as u128)
        } else {
            0
        };
        
        // Simple lending rate model
        // Base yield is 80% of the borrow rate, scaled by utilization
        let lending_rate = utilization_rate
            .checked_mul(80)
            .unwrap_or(0)
            .checked_div(100)
            .unwrap_or(0);
            
        // Update cumulative lending rate
        // Formula: previous_rate + (lending_rate * time_elapsed / SECONDS_PER_YEAR)
        const SECONDS_PER_YEAR: u128 = 31536000; // 365 * 24 * 60 * 60
        
        let rate_increase = lending_rate
            .checked_mul(time_elapsed)
            .unwrap_or(0)
            .checked_div(SECONDS_PER_YEAR)
            .unwrap_or(0);
            
        // Update pool's cumulative lending rate
        // If this is the first update, initialize with 1 * 10^12 as base value
        if pool.cumulative_lending_rate == 0 {
            pool.cumulative_lending_rate = 1_000_000_000_000;
        }
        
        pool.cumulative_lending_rate = pool.cumulative_lending_rate
            .checked_add(rate_increase)
            .unwrap_or(pool.cumulative_lending_rate);
            
        // Update timestamp
        pool.last_updated = current_timestamp;
        
        Ok(())
    }
    
    /// Check if a user has any lending positions enabled
    pub fn has_lending_positions(
        user_position: &UserPosition,
    ) -> bool {
        for collateral in &user_position.collaterals {
            if collateral.is_lending {
                return true;
            }
        }
        false
    }
    
    /// Check if a specific deposit is being used for lending
    pub fn is_lending_enabled(
        user_position: &UserPosition,
        pool_key: &Pubkey
    ) -> bool {
        for collateral in &user_position.collaterals {
            if &collateral.pool == pool_key && collateral.is_lending {
                return true;
            }
        }
        false
    }
    
    /// Enable or disable lending for a specific deposit
    pub fn set_lending_status<'a>(
        user_position: &mut Account<'a, UserPosition>,
        pool_key: &Pubkey,
        enable_lending: bool,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<()> {
        let mut found = false;
        
        for collateral in &mut user_position.collaterals {
            if collateral.pool == *pool_key {
                collateral.is_lending = enable_lending;
                found = true;
                break;
            }
        }
        
        require!(found, OxygenError::CollateralNotFound);
        
        // Recalculate health factor after change in case this affects borrowing capacity
        let _ = user_position.calculate_health_factor(pool_data)?;
        
        Ok(())
    }
}