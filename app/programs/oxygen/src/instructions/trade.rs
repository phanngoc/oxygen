use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition, MarketInfo};
use crate::errors::OxygenError;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TradeParams {
    pub size: u64,               // Size of the order
    pub price: u64,              // Limit price
    pub side: OrderSide,         // Buy or sell
    pub order_type: OrderType,   // Limit or market
    pub leverage: u64,           // Leverage multiplier (e.g. 20000 = 2x)
    pub time_in_force: u16,      // Time in force
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum OrderType {
    Limit,
    Market,
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
        constraint = base_asset_reserve.mint == base_asset_pool.asset_mint,
    )]
    pub base_asset_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
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
    
    // In a full implementation, we would include Serum market accounts
    // such as the orderbook, event queue, request queue, etc.
    // For simplicity in this MVP scaffold, we're omitting those
    
    pub token_program: Program<'info, anchor_spl::token::Token>,
    // The Serum program would also be included here in a full implementation
}

pub fn handler(ctx: Context<TradeWithLeverage>, params: TradeParams) -> Result<()> {
    // Validate parameters
    require!(params.size > 0, OxygenError::InvalidParameter);
    require!(params.price > 0, OxygenError::InvalidParameter);
    
    let market_info = &ctx.accounts.market_info;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Check if requested leverage is within allowed limits
    require!(
        params.leverage <= market_info.max_leverage,
        OxygenError::LeverageExceedsMaximum
    );
    
    // Calculate required margin based on order size, price and leverage
    let position_notional = (params.size as u128)
        .checked_mul(params.price as u128)
        .ok_or(ErrorCode::MathOverflow)?;
        
    let required_margin = position_notional
        .checked_mul(10000) // Base scale
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(params.leverage as u128)
        .ok_or(ErrorCode::MathOverflow)? as u64;
    
    // In a full implementation, we would:
    // 1. Calculate if the user has enough cross-collateral to support this trade
    // 2. Reserve the margin from the user's available collateral
    // 3. Place the order on the Serum DEX
    // 4. Update the user's position with the new leveraged trade
    // 5. Set up monitoring for liquidation thresholds
    
    // For this MVP scaffold, we'll just perform validation checks
    
    // Mock cross-collateral validation
    // In a real implementation, this would check all user's collateral across pools
    let mut pool_data = HashMap::new();
    
    // Mock prices and liquidation thresholds - would come from oracles in a real implementation
    pool_data.insert(ctx.accounts.base_asset_pool.key(), (10000, ctx.accounts.base_asset_pool.liquidation_threshold));
    pool_data.insert(ctx.accounts.quote_asset_pool.key(), (10000, ctx.accounts.quote_asset_pool.liquidation_threshold));
    
    // Calculate health factor with the trade
    // Note: In a real implementation, we would simulate adding the leverage position
    // to the user's account first before checking the health factor
    let health_factor = user_position.calculate_health_factor(&pool_data)?;
    
    const MIN_HEALTH_FACTOR: u64 = 12000; // Slightly higher for leverage trading (1.2x)
    require!(
        health_factor >= MIN_HEALTH_FACTOR,
        OxygenError::HealthFactorTooLow
    );
    
    // In a complete implementation, this is where we would:
    // 1. Create open orders account if needed
    // 2. Place order on Serum DEX
    // 3. Record the leveraged position in the user's account
    
    msg!("Validated leveraged trade: {} @ {} with {}x leverage",
        params.size,
        params.price,
        params.leverage as f64 / 10000.0
    );
    
    // In an actual implementation, we would update the user's position data
    // with the new leveraged trade details
    user_position.last_updated = clock.unix_timestamp;
    
    Ok(())
}