use anchor_lang::prelude::*;
use crate::state::{MarketInfo, UserPosition, Pool};
use crate::errors::OxygenError;
use crate::instructions::{OrderSide, OrderType};
use std::collections::HashMap;

/// Module for handling trading operations with Serum DEX
pub struct TradingModule;

impl TradingModule {
    /// Validate if a trade can be executed with given leverage
    pub fn validate_leveraged_trade(
        user_position: &UserPosition,
        market_info: &MarketInfo,
        base_pool: &Pool,
        quote_pool: &Pool,
        size: u64,
        price: u64,
        leverage: u64,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<()> {
        // Check if leverage is within allowed limits
        require!(
            leverage <= market_info.max_leverage,
            OxygenError::LeverageExceedsMaximum
        );
        
        // Calculate position value
        let position_value = (size as u128)
            .checked_mul(price as u128)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Calculate required margin
        let required_margin = position_value
            .checked_mul(10000) // Base scale factor
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(leverage as u128)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        // Check if user has enough collateral to support this position
        let collateral_value = Self::calculate_user_available_collateral(
            user_position,
            pool_data
        )?;
        
        require!(
            collateral_value >= required_margin as u128,
            OxygenError::InsufficientCollateral
        );
        
        // Additional checks for liquidation risk
        const MIN_LEVERAGE_HEALTH_FACTOR: u64 = 12000; // 1.2 in basis points, higher than regular lending
        
        // Simulate health factor with this position
        let health_factor = Self::simulate_position_health_factor(
            user_position,
            pool_data,
            position_value,
            required_margin as u128
        )?;
        
        require!(
            health_factor >= MIN_LEVERAGE_HEALTH_FACTOR,
            OxygenError::HealthFactorTooLow
        );
        
        Ok(())
    }
    
    /// Calculate user's available collateral for trading
    fn calculate_user_available_collateral(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<u128> {
        let mut total_available = 0u128;
        
        for collateral in &user_position.collaterals {
            if !collateral.is_collateral {
                continue;
            }
            
            if let Some((price, _)) = pool_data.get(&collateral.pool) {
                let value = (collateral.amount_deposited as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                total_available = total_available
                    .checked_add(value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        // Subtract any amounts already being used as collateral for loans
        // (In a real implementation, this would be more sophisticated)
        let mut borrowed_value = 0u128;
        
        for borrow in &user_position.borrows {
            if let Some((price, _)) = pool_data.get(&borrow.pool) {
                let value = (borrow.amount_borrowed as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                borrowed_value = borrowed_value
                    .checked_add(value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        // Apply a conservative factor for trading margin
        // Only 80% of excess collateral can be used for trading
        if borrowed_value >= total_available {
            return Ok(0);
        }
        
        let excess_collateral = total_available
            .checked_sub(borrowed_value)
            .ok_or(ErrorCode::MathOverflow)?;
            
        let trading_available = excess_collateral
            .checked_mul(80)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(100)
            .ok_or(ErrorCode::MathOverflow)?;
            
        Ok(trading_available)
    }
    
    /// Simulate health factor with a new trading position
    fn simulate_position_health_factor(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)>,
        position_value: u128,
        margin_used: u128
    ) -> Result<u64> {
        // For a trade, we simulate as if:
        // 1. The margin is being used (reduced from available collateral)
        // 2. The leveraged position adds risk comparable to a loan
        
        // Calculate current weighted collateral value
        let mut weighted_collateral_value = 0u128;
        
        for collateral in &user_position.collaterals {
            if !collateral.is_collateral {
                continue;
            }
            
            if let Some((price, liquidation_threshold)) = pool_data.get(&collateral.pool) {
                let value = (collateral.amount_deposited as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                let weighted_value = value
                    .checked_mul(*liquidation_threshold as u128)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(10000)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                weighted_collateral_value = weighted_collateral_value
                    .checked_add(weighted_value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        // Calculate current borrowed value
        let mut borrowed_value = 0u128;
        
        for borrow in &user_position.borrows {
            if let Some((price, _)) = pool_data.get(&borrow.pool) {
                let value = (borrow.amount_borrowed as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                borrowed_value = borrowed_value
                    .checked_add(value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        // Adjust collateral for margin used
        let adjusted_weighted_collateral = if weighted_collateral_value > margin_used {
            weighted_collateral_value
                .checked_sub(margin_used)
                .ok_or(ErrorCode::MathOverflow)?
        } else {
            0
        };
        
        // Treat position as additional "borrowed" value for risk calculation
        let total_risk_value = borrowed_value
            .checked_add(position_value)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Calculate simulated health factor
        if total_risk_value == 0 {
            return Ok(u64::MAX); // No risk
        }
        
        let health_factor = adjusted_weighted_collateral
            .checked_mul(10000)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(total_risk_value)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        Ok(health_factor)
    }
    
    /// Create a mock serum order (for the MVP scaffold)
    /// In a real implementation, this would interact with the Serum DEX
    pub fn create_order(
        user: &Pubkey,
        market: &Pubkey,
        side: OrderSide,
        order_type: OrderType,
        size: u64,
        price: u64,
        leverage: u64,
        client_id: u64
    ) -> Result<()> {
        // Log the order data (this is just a placeholder for the MVP)
        msg!(
            "Order created: User={}, Market={}, Side={:?}, Type={:?}, Size={}, Price={}, Leverage={}x, ClientID={}",
            user,
            market,
            side,
            order_type,
            size,
            price,
            leverage as f64 / 10000.0,
            client_id
        );
        
        // In a real implementation, this would:
        // 1. Create an open orders account if needed
        // 2. Place an order on Serum DEX
        // 3. Record the leverage position in the user's account
        
        Ok(())
    }
}