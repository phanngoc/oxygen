use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition, MarketInfo};
use crate::errors::OxygenError;
use crate::modules::trading::TradingModule;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TradeParams {
    pub size: u64,               // Size of the order in base asset
    pub price: u64,              // Limit price
    pub side: OrderSide,         // Buy or sell
    pub order_type: OrderType,   // Limit or market
    pub leverage: u64,           // Leverage multiplier (e.g. 20000 = 2x)
    pub client_id: u64,          // Client order ID for tracking
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ClosePositionParams {
    pub position_id: u64,        // ID of the position to close
    pub price: u64,              // Execution price
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub enum OrderSide {
    #[default]
    Buy,
    Sell,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub enum OrderType {
    #[default]
    Limit,
    Market,
    // Could add IOC, PostOnly, etc. for a complete implementation
}

#[derive(Accounts)]
pub struct TradeWithLeverage<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        seeds = [b"market", market_info.serum_market.as_ref()],
        bump = market_info.bump,
    )]
    pub market_info: Account<'info, MarketInfo>,
    
    #[account(
        mut,
        seeds = [b"pool", base_asset_pool.asset_mint.as_ref()],
        bump = base_asset_pool.bump,
    )]
    pub base_asset_pool: Account<'info, Pool>,
    
    #[account(
        mut,
        seeds = [b"pool", quote_asset_pool.asset_mint.as_ref()],
        bump = quote_asset_pool.bump,
    )]
    pub quote_asset_pool: Account<'info, Pool>,
    
    #[account(
        mut,
        seeds = [b"reserve", base_asset_pool.key().as_ref()],
        bump,
        constraint = base_asset_reserve.mint == base_asset_pool.asset_mint,
    )]
    pub base_asset_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"reserve", quote_asset_pool.key().as_ref()],
        bump,
        constraint = quote_asset_reserve.mint == quote_asset_pool.asset_mint,
    )]
    pub quote_asset_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"position", user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key(),
    )]
    pub user_position: Account<'info, UserPosition>,
    
    // In a full implementation, we would include these Serum market accounts:
    // pub serum_market: Account<'info, serum_dex::Market>,
    // pub serum_request_queue: Account<'info, serum_dex::RequestQueue>,
    // pub serum_event_queue: Account<'info, serum_dex::EventQueue>,
    // pub serum_bids: Account<'info, serum_dex::Bids>,
    // pub serum_asks: Account<'info, serum_dex::Asks>,
    // pub serum_coin_vault: Account<'info, TokenAccount>,
    // pub serum_pc_vault: Account<'info, TokenAccount>,
    // #[account(mut)]
    // pub open_orders: Account<'info, serum_dex::OpenOrders>,
    
    pub token_program: Program<'info, anchor_spl::token::Token>,
    // pub dex_program: Program<'info, serum_dex::Dex>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct CloseTradePosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"position", user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key(),
    )]
    pub user_position: Account<'info, UserPosition>,
    
    #[account(
        seeds = [b"market", market_info.serum_market.as_ref()],
        bump = market_info.bump,
    )]
    pub market_info: Account<'info, MarketInfo>,
    
    #[account(
        mut,
        seeds = [b"pool", base_asset_pool.asset_mint.as_ref()],
        bump = base_asset_pool.bump,
    )]
    pub base_asset_pool: Account<'info, Pool>,
    
    #[account(
        mut,
        seeds = [b"pool", quote_asset_pool.asset_mint.as_ref()],
        bump = quote_asset_pool.bump,
    )]
    pub quote_asset_pool: Account<'info, Pool>,
    
    // Similar to open trade, we would include Serum market accounts here
    // for a complete implementation
    
    pub token_program: Program<'info, anchor_spl::token::Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn open_trade(ctx: Context<TradeWithLeverage>, params: TradeParams) -> Result<()> {
    // Validate parameters
    require!(params.size > 0, OxygenError::InvalidParameter);
    require!(params.price > 0, OxygenError::InvalidParameter);
    require!(params.leverage >= 10000, OxygenError::InvalidParameter); // Min 1x leverage
    
    let market_info = &ctx.accounts.market_info;
    let user_position = &mut ctx.accounts.user_position;
    let base_pool = &ctx.accounts.base_asset_pool;
    let quote_pool = &ctx.accounts.quote_asset_pool;
    
    // Calculate the notional value of the position
    let position_value = (params.size as u128)
        .checked_mul(params.price as u128)
        .ok_or(ErrorCode::MathOverflow)? as u64;
        
    // Calculate required margin
    let required_margin = position_value
        .checked_mul(10000) // Base scale factor (10000 = 1x)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(params.leverage)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Mock price data for health factor calculation
    // In a real implementation, this would come from oracles
    let mut pool_data = HashMap::new();
    pool_data.insert(base_pool.key(), (10000, base_pool.liquidation_threshold));
    pool_data.insert(quote_pool.key(), (10000, quote_pool.liquidation_threshold));
    
    // Create open orders account if it doesn't exist yet
    // In a real implementation, we would check if the user already has an open orders account
    // for this market and create one if needed
    let _ = TradingModule::initialize_open_orders_account(&ctx)?;
    
    // 1. Lock the required margin from the user's collateral
    TradingModule::lock_margin_from_collateral(
        user_position,
        required_margin,
        &pool_data
    )?;
    
    // Create the order on Serum DEX
    let position_id = TradingModule::create_order(
        &ctx.accounts.user.key(),
        &market_info.serum_market,
        market_info,
        base_pool,
        quote_pool,
        user_position,
        params.side,
        params.order_type,
        params.size,
        params.price,
        params.leverage,
        params.client_id,
        &pool_data
    )?;
    
    // 2. Place the actual order on Serum DEX
    TradingModule::place_serum_dex_order(
        &ctx,
        market_info,
        params.side,
        params.order_type,
        params.size,
        params.price,
        params.client_id
    )?;
    
    // 3. Set up monitoring for position health
    // Note: This is already done inside the create_order function
    
    // Update the user's health factor with the new position
    user_position.calculate_health_factor(&pool_data)?;
    user_position.last_updated = ctx.accounts.clock.unix_timestamp;
    
    msg!("Opened leveraged trade position {}: {} {} @ {} with {}x leverage",
        position_id,
        params.size,
        match params.side {
            OrderSide::Buy => "Buy",
            OrderSide::Sell => "Sell",
        },
        params.price,
        params.leverage as f64 / 10000.0
    );
    
    Ok(())
}

pub fn close_position(ctx: Context<CloseTradePosition>, params: ClosePositionParams) -> Result<()> {
    let user_position = &mut ctx.accounts.user_position;
    
    // Mock price data for health factor calculation
    let mut pool_data = HashMap::new();
    pool_data.insert(ctx.accounts.base_asset_pool.key(), 
        (10000, ctx.accounts.base_asset_pool.liquidation_threshold));
    pool_data.insert(ctx.accounts.quote_asset_pool.key(), 
        (10000, ctx.accounts.quote_asset_pool.liquidation_threshold));
    
    // Close the position
    TradingModule::close_position(
        user_position,
        params.position_id,
        params.price,
        &pool_data
    )?;
    
    // In a full implementation, we would:
    // 1. Place a counter order on Serum DEX to close the position
    // 2. Return the locked margin to the user's available collateral
    // 3. Apply the PnL to the user's balance
    
    // Update user position's health factor
    user_position.calculate_health_factor(&pool_data)?;
    user_position.last_updated = ctx.accounts.clock.unix_timestamp;
    
    msg!("Closed leveraged position {} at price {}", params.position_id, params.price);
    
    Ok(())
}

/// Monitor open leveraged positions and liquidate if necessary
pub fn monitor_positions_for_liquidation<'info>(
    ctx: Context<'_, '_, '_, 'info>, 
    current_prices: HashMap<Pubkey, u64>
) -> Result<()> {
    // Extract the user position to monitor
    let user_position = &mut ctx.accounts.user_position;
    
    // Mock price data for health factor calculation
    let mut pool_data = HashMap::new();
    
    // In a real implementation, we would:
    // 1. Add all pool data from oracles
    // 2. Monitor positions across multiple users
    
    // Add some mock data for the example
    for (market, price) in &current_prices {
        pool_data.insert(*market, (*price, 8000)); // 80% liquidation threshold
    }
    
    // Monitor and potentially liquidate positions
    TradingModule::monitor_positions(
        user_position,
        &current_prices,
        &pool_data
    )?;
    
    // Update user position's health factor after any liquidations
    user_position.calculate_health_factor(&pool_data)?;
    
    Ok(())
}

/// Process the funding rate adjustments for open leveraged positions
pub fn process_funding_rates<'info>(
    ctx: Context<'_, '_, '_, 'info>,
    funding_rates: HashMap<Pubkey, i64>  // Positive = longs pay shorts, negative = shorts pay longs
) -> Result<()> {
    // Extract the user position to monitor
    let user_position = &mut ctx.accounts.user_position;
    
    // Process funding payments for each open position
    for position in &mut user_position.leveraged_positions {
        if let Some(&rate) = funding_rates.get(&position.market) {
            // Skip closed positions
            if position.status != crate::state::PositionStatus::Open {
                continue;
            }
            
            // Calculate funding amount based on position size and rate
            // rate is in basis points per hour (e.g. 1 = 0.01% per hour)
            let funding_amount = (position.position_value as i128)
                .checked_mul(rate as i128)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(1_000_000) // 10000 (bps) * 100 (percent)
                .ok_or(ErrorCode::MathOverflow)? as i64;
            
            // Apply funding
            // Positive funding: longs pay shorts
            // Negative funding: shorts pay longs
            let funding_direction = match position.side {
                OrderSide::Buy => -funding_amount, // Longs pay when positive rate
                OrderSide::Sell => funding_amount, // Shorts receive when positive rate
            };
            
            msg!("Position {} funding payment: {}", position.id, funding_direction);
            
            // In a real implementation, we would actually transfer the funds
            // between longs and shorts in the protocol
        }
    }
    
    Ok(())
}

/// Get user's open leveraged positions
pub fn get_open_positions<'info>(ctx: Context<'_, '_, '_, 'info>) -> Result<Vec<u64>> {
    let user_position = &ctx.accounts.user_position;
    
    let mut open_positions = Vec::new();
    for position in &user_position.leveraged_positions {
        if position.status == crate::state::PositionStatus::Open {
            open_positions.push(position.id);
        }
    }
    
    msg!("User has {} open positions", open_positions.len());
    
    Ok(open_positions)
}