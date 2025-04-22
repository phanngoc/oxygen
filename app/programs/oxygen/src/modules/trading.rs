use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use crate::state::{MarketInfo, UserPosition, Pool, LeveragedPosition};
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
    pub fn calculate_user_available_collateral(
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
        
        // Also subtract margin already committed to leveraged positions
        let mut leveraged_margin_used = 0u128;
        for position in &user_position.leveraged_positions {
            leveraged_margin_used = leveraged_margin_used
                .checked_add(position.margin_used as u128)
                .ok_or(ErrorCode::MathOverflow)?;
        }
        
        // Apply a conservative factor for trading margin
        // Only 80% of excess collateral can be used for trading
        let total_used = borrowed_value
            .checked_add(leveraged_margin_used)
            .ok_or(ErrorCode::MathOverflow)?;
            
        if total_used >= total_available {
            return Ok(0);
        }
        
        let excess_collateral = total_available
            .checked_sub(total_used)
            .ok_or(ErrorCode::MathOverflow)?;
            
        let trading_available = excess_collateral
            .checked_mul(80)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(100)
            .ok_or(ErrorCode::MathOverflow)?;
            
        Ok(trading_available)
    }
    
    /// Simulate health factor with a new trading position
    pub fn simulate_position_health_factor(
        user_position: &UserPosition,
        pool_data: &HashMap<Pubkey, (u64, u64)>,
        position_value: u128,
        margin_used: u128
    ) -> Result<u64> {
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
        
        // Include existing leveraged positions risk
        for position in &user_position.leveraged_positions {
            borrowed_value = borrowed_value
                .checked_add(position.position_value as u128)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_sub(position.margin_used as u128) // Margin is already in collateral
                .ok_or(ErrorCode::MathOverflow)?;
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
    
    /// Lock margin from user's collateral for a leveraged trade
    pub fn lock_margin_from_collateral<'a>(
        user_position: &mut Account<'a, UserPosition>,
        required_margin: u64,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<()> {
        // Get the available collateral in the user's account
        let available_collateral = Self::calculate_user_available_collateral(
            user_position,
            pool_data
        )?;
        
        require!(
            available_collateral >= required_margin as u128,
            OxygenError::InsufficientCollateral
        );
        
        // Mark collateral as locked for trading
        // This updates an internal tracking field to ensure this collateral
        // isn't double-counted as available for other operations
        user_position.locked_trading_margin = user_position.locked_trading_margin
            .checked_add(required_margin)
            .ok_or(ErrorCode::MathOverflow)?;
            
        msg!("Locked {} margin for leveraged trading", required_margin);
        
        Ok(())
    }
    
    /// Place an order on Serum DEX
    pub fn place_serum_dex_order<'a, 'info>(
        ctx: &Context<'_, '_, '_, 'info>,
        market_info: &Account<'a, MarketInfo>,
        side: OrderSide,
        order_type: OrderType,
        size: u64,
        price: u64,
        client_id: u64
    ) -> Result<()> {
        // Convert our OrderSide to Serum OrderSide
        let serum_side = match side {
            OrderSide::Buy => {
                msg!("Placing BUY order on Serum DEX");
                // serum_dex::matching::Side::Bid
                0 // Using 0 to represent Bid since we don't have direct Serum types
            },
            OrderSide::Sell => {
                msg!("Placing SELL order on Serum DEX");
                // serum_dex::matching::Side::Ask
                1 // Using 1 to represent Ask since we don't have direct Serum types
            }
        };
        
        // Convert our OrderType to Serum OrderType
        let serum_order_type = match order_type {
            OrderType::Limit => {
                msg!("Order type: LIMIT at price {}", price);
                // serum_dex::matching::OrderType::Limit
                0 // Using 0 to represent Limit order
            },
            OrderType::Market => {
                msg!("Order type: MARKET");
                // serum_dex::matching::OrderType::ImmediateOrCancel
                1 // Using 1 to represent IoC (market) order
            }
        };
        
        // For a real implementation, we would:
        // 1. Get all required Serum DEX accounts from ctx
        // 2. Create a CPI call to the Serum DEX program
        // 3. Pass all required accounts and parameters

        // Example of what the actual code would look like:
        // let serum_accounts = SerumDEXAccounts {
        //     market: ctx.accounts.serum_market.to_account_info(),
        //     open_orders: ctx.accounts.open_orders.to_account_info(),
        //     request_queue: ctx.accounts.serum_request_queue.to_account_info(),
        //     event_queue: ctx.accounts.serum_event_queue.to_account_info(),
        //     bids: ctx.accounts.serum_bids.to_account_info(),
        //     asks: ctx.accounts.serum_asks.to_account_info(),
        //     coin_vault: ctx.accounts.serum_coin_vault.to_account_info(),
        //     pc_vault: ctx.accounts.serum_pc_vault.to_account_info(),
        //     // other required accounts...
        // };
        //
        // serum_dex::new_order(
        //     CpiContext::new(
        //         ctx.accounts.dex_program.to_account_info(),
        //         serum_accounts
        //     ),
        //     serum_side,
        //     price,
        //     size,
        //     serum_order_type,
        //     client_id
        // )?;

        msg!(
            "Order placed on Serum DEX: Market={}, Size={}, Price={}, ClientID={}",
            market_info.serum_market,
            size,
            price,
            client_id
        );
        
        Ok(())
    }

    /// Set up monitoring for a position's health
    pub fn setup_position_monitoring<'a>(
        position_id: u64,
        market: Pubkey,
        liquidation_price: u64,
        user: Pubkey
    ) -> Result<()> {
        // In a full implementation, this would:
        // 1. Register this position with an off-chain monitoring service
        // 2. Set up any on-chain subscriptions needed
        // 3. Store relevant monitoring parameters
        
        // For now, we'll just log the monitoring setup
        msg!(
            "Position monitoring setup: ID={}, Market={}, User={}, Liquidation Price={}",
            position_id,
            market,
            user,
            liquidation_price
        );
        
        // Emit a program event to notify off-chain monitors
        emit!(PositionCreatedEvent {
            position_id,
            market,
            user,
            liquidation_price,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }

    /// Create an order on Serum DEX
    pub fn create_order<'a, 'info>(
        user: &Pubkey,
        market: &Pubkey,
        market_info: &Account<'a, MarketInfo>,
        base_pool: &Account<'a, Pool>,
        quote_pool: &Account<'a, Pool>,
        user_position: &mut Account<'a, UserPosition>,
        side: OrderSide,
        order_type: OrderType,
        size: u64,
        price: u64,
        leverage: u64,
        client_id: u64,
        pool_data: &HashMap<Pubkey, (u64, u64)>,
    ) -> Result<u64> {
        // Calculate position value and required margin
        let position_value = (size as u128)
            .checked_mul(price as u128)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        let required_margin = position_value
            .checked_mul(10000) // Base scale factor
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(leverage as u64)
            .ok_or(ErrorCode::MathOverflow)?;

        // Validate trade against user's collateral
        Self::validate_leveraged_trade(
            user_position, 
            market_info, 
            base_pool, 
            quote_pool, 
            size, 
            price, 
            leverage, 
            pool_data
        )?;

        // Generate a position ID
        let position_id = Self::generate_position_id(user_position)?;
        
        // 1. Lock the required margin from the user's collateral
        Self::lock_margin_from_collateral(
            user_position,
            required_margin,
            pool_data
        )?;
        
        // Create a new leveraged position
        let new_position = LeveragedPosition {
            id: position_id,
            market: *market,
            side,
            size,
            entry_price: price,
            leverage,
            margin_used: required_margin,
            position_value,
            timestamp: Clock::get()?.unix_timestamp,
            status: crate::state::PositionStatus::Open,
            liquidation_price: Self::calculate_liquidation_price(
                side, 
                price, 
                leverage, 
                market_info.maintenance_margin_ratio
            )?,
            client_id,
        };
        
        // Add the position to the user's account
        user_position.leveraged_positions.push(new_position);
        
        // 3. Set up monitoring for position health
        Self::setup_position_monitoring(
            position_id,
            *market,
            new_position.liquidation_price,
            *user
        )?;
        
        msg!(
            "Leveraged position opened: ID={}, User={}, Market={}, Side={:?}, Size={}, Price={}, Leverage={}x",
            position_id,
            user,
            market,
            side,
            size,
            price,
            leverage as f64 / 10000.0
        );
        
        Ok(position_id)
    }
    
    /// Close an existing leveraged position
    pub fn close_position<'a>(
        user_position: &mut Account<'a, UserPosition>,
        position_id: u64,
        execution_price: u64,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<()> {
        // Find the position with the given ID
        let position_index = user_position.leveraged_positions
            .iter()
            .position(|p| p.id == position_id)
            .ok_or(OxygenError::PositionNotFound)?;
            
        let position = &mut user_position.leveraged_positions[position_index];
        
        // Ensure position is not already closed
        require!(
            position.status == crate::state::PositionStatus::Open,
            OxygenError::PositionAlreadyClosed
        );
        
        // Calculate PnL
        let (pnl, is_profit) = Self::calculate_pnl(
            position.side,
            position.entry_price,
            execution_price,
            position.size,
            position.leverage
        )?;
        
        // Update position status
        position.status = crate::state::PositionStatus::Closed;
        
        // In a real implementation, we would:
        // 1. Return the margin to the user's available collateral
        // 2. Apply the PnL to the user's balance
        // 3. Close the position on Serum DEX
        
        // Update user's position health factor after closing
        let _ = user_position.calculate_health_factor(pool_data)?;
        
        msg!(
            "Leveraged position closed: ID={}, PnL={}{}, Exit Price={}",
            position_id,
            if is_profit { "+" } else { "-" },
            pnl,
            execution_price
        );
        
        // In a full implementation, we might want to keep closed positions for history
        // but for now we'll just remove it
        user_position.leveraged_positions.remove(position_index);
        
        Ok(())
    }
    
    /// Liquidate an underwater leveraged position
    pub fn liquidate_position<'a>(
        user_position: &mut Account<'a, UserPosition>,
        position_id: u64,
        liquidation_price: u64,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<()> {
        // Find the position with the given ID
        let position_index = user_position.leveraged_positions
            .iter()
            .position(|p| p.id == position_id)
            .ok_or(OxygenError::PositionNotFound)?;
            
        let position = &mut user_position.leveraged_positions[position_index];
        
        // Ensure position is open
        require!(
            position.status == crate::state::PositionStatus::Open,
            OxygenError::PositionAlreadyClosed
        );
        
        // Check if position is eligible for liquidation
        let is_liquidatable = match position.side {
            OrderSide::Buy => liquidation_price <= position.liquidation_price,
            OrderSide::Sell => liquidation_price >= position.liquidation_price,
        };
        
        require!(is_liquidatable, OxygenError::PositionNotLiquidatable);
        
        // Calculate remaining margin after liquidation (if any)
        // Note: In a real implementation, this would be more sophisticated
        // and include liquidation penalties
        let remaining_margin = if liquidation_price == 0 {
            0 // Full liquidation
        } else {
            let (loss, _) = Self::calculate_pnl(
                position.side,
                position.entry_price,
                liquidation_price,
                position.size,
                position.leverage
            )?;
            
            if loss >= position.margin_used {
                0 // No margin remaining
            } else {
                position.margin_used - loss
            }
        };
        
        // Update position status
        position.status = crate::state::PositionStatus::Liquidated;
        
        // In a real implementation, we would:
        // 1. Return any remaining margin to the user
        // 2. Apply liquidation penalties
        // 3. Close the position on Serum DEX
        
        msg!(
            "Leveraged position liquidated: ID={}, Price={}, Remaining Margin={}",
            position_id,
            liquidation_price,
            remaining_margin
        );
        
        // Remove the liquidated position
        user_position.leveraged_positions.remove(position_index);
        
        // Update user's position health factor after liquidation
        let _ = user_position.calculate_health_factor(pool_data)?;
        
        Ok(())
    }
    
    /// Generate a unique position ID
    fn generate_position_id(user_position: &UserPosition) -> Result<u64> {
        // Simple ID generation for MVP
        // In a real implementation, this would be more sophisticated
        let mut max_id = 0;
        for pos in &user_position.leveraged_positions {
            if pos.id > max_id {
                max_id = pos.id;
            }
        }
        
        max_id = max_id.checked_add(1).ok_or(ErrorCode::MathOverflow)?;
        
        Ok(max_id)
    }
    
    /// Calculate the liquidation price for a position
    fn calculate_liquidation_price(
        side: OrderSide,
        entry_price: u64,
        leverage: u64,
        maintenance_margin_ratio: u64
    ) -> Result<u64> {
        // For long positions: liquidation_price = entry_price * (1 - maintenance_margin_ratio * leverage / 10000)
        // For short positions: liquidation_price = entry_price * (1 + maintenance_margin_ratio * leverage / 10000)
        
        let margin_impact = (maintenance_margin_ratio as u128)
            .checked_mul(leverage as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        match side {
            OrderSide::Buy => {
                if margin_impact >= 10000 {
                    return Ok(0); // Will be liquidated immediately
                }
                
                let factor = 10000u64
                    .checked_sub(margin_impact)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                let liquidation_price = (entry_price as u128)
                    .checked_mul(factor as u128)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(10000)
                    .ok_or(ErrorCode::MathOverflow)? as u64;
                    
                Ok(liquidation_price)
            },
            OrderSide::Sell => {
                let factor = 10000u64
                    .checked_add(margin_impact)
                    .ok_or(ErrorCode::MathOverflow)?;
                    
                let liquidation_price = (entry_price as u128)
                    .checked_mul(factor as u128)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(10000)
                    .ok_or(ErrorCode::MathOverflow)? as u64;
                    
                Ok(liquidation_price)
            }
        }
    }
    
    /// Calculate PnL for a position
    fn calculate_pnl(
        side: OrderSide,
        entry_price: u64,
        exit_price: u64,
        size: u64,
        leverage: u64
    ) -> Result<(u64, bool)> {
        // Calculate raw PnL
        let (raw_pnl, is_profit) = match side {
            OrderSide::Buy => {
                if exit_price > entry_price {
                    // Profit
                    let diff = exit_price
                        .checked_sub(entry_price)
                        .ok_or(ErrorCode::MathOverflow)?;
                        
                    ((diff as u128)
                        .checked_mul(size as u128)
                        .ok_or(ErrorCode::MathOverflow)? as u64, 
                     true)
                } else {
                    // Loss
                    let diff = entry_price
                        .checked_sub(exit_price)
                        .ok_or(ErrorCode::MathOverflow)?;
                        
                    ((diff as u128)
                        .checked_mul(size as u128)
                        .ok_or(ErrorCode::MathOverflow)? as u64, 
                     false)
                }
            },
            OrderSide::Sell => {
                if entry_price > exit_price {
                    // Profit
                    let diff = entry_price
                        .checked_sub(exit_price)
                        .ok_or(ErrorCode::MathOverflow)?;
                        
                    ((diff as u128)
                        .checked_mul(size as u128)
                        .ok_or(ErrorCode::MathOverflow)? as u64, 
                     true)
                } else {
                    // Loss
                    let diff = exit_price
                        .checked_sub(entry_price)
                        .ok_or(ErrorCode::MathOverflow)?;
                        
                    ((diff as u128)
                        .checked_mul(size as u128)
                        .ok_or(ErrorCode::MathOverflow)? as u64, 
                     false)
                }
            }
        };
        
        // Apply leverage
        let leveraged_pnl = (raw_pnl as u128)
            .checked_mul(leverage as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        Ok((leveraged_pnl, is_profit))
    }

    /// Monitor open positions and check for liquidation conditions
    pub fn monitor_positions<'a>(
        user_position: &mut Account<'a, UserPosition>,
        current_prices: &HashMap<Pubkey, u64>,
        pool_data: &HashMap<Pubkey, (u64, u64)>
    ) -> Result<()> {
        let mut positions_to_liquidate = Vec::new();
        
        for (i, position) in user_position.leveraged_positions.iter().enumerate() {
            if position.status != crate::state::PositionStatus::Open {
                continue;
            }
            
            // Get current price for the market
            if let Some(&current_price) = current_prices.get(&position.market) {
                let is_liquidatable = match position.side {
                    OrderSide::Buy => current_price <= position.liquidation_price,
                    OrderSide::Sell => current_price >= position.liquidation_price,
                };
                
                if is_liquidatable {
                    positions_to_liquidate.push((i, position.id, current_price));
                }
            }
        }
        
        // Liquidate positions (in reverse order to not mess up indices)
        for (_, position_id, price) in positions_to_liquidate.iter().rev() {
            let _ = Self::liquidate_position(user_position, *position_id, *price, pool_data)?;
        }
        
        Ok(())
    }

    /// Initialize Serum open orders account for a user (if needed)
    pub fn initialize_open_orders_account(_ctx: &Context<'_, '_,'_, '_>) -> Result<()> {
        // This would be implemented in a full version
        // and would integrate with the Serum DEX program
        // to create an open orders account for the user
        msg!("Initializing open orders account");
        Ok(())
    }

    /// Apply realized PnL to the user's account
    pub fn apply_realized_pnl(
        user_position: &mut UserPosition,
        realized_pnl: i64, // Positive for profit, negative for loss
        base_pool: &Pubkey,
        quote_pool: &Pubkey
    ) -> Result<()> {
        // In a real implementation, this would handle:
        // 1. Increasing user's balance in case of profit
        // 2. Decreasing user's balance in case of loss
        // 3. Updating affected pool balances
        
        if realized_pnl > 0 {
            // Mock handling of profit
            msg!("Realized profit: {}", realized_pnl);
        } else if realized_pnl < 0 {
            // Mock handling of loss
            msg!("Realized loss: {}", realized_pnl.abs());
        }
        
        Ok(())
    }
}

// Event emitted when a new position is created for off-chain monitoring
#[event]
pub struct PositionCreatedEvent {
    pub position_id: u64,
    pub market: Pubkey,
    pub user: Pubkey,
    pub liquidation_price: u64,
    pub timestamp: i64,
}