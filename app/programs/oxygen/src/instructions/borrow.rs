use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct BorrowParams {
    pub amount: u64,  // Amount to borrow
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"pool", pool.asset_mint.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,
    
    #[account(
        mut,
        constraint = user_token_account.mint == pool.asset_mint,
        constraint = user_token_account.owner == user.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"reserve", pool.key().as_ref()],
        bump,
        constraint = asset_reserve.mint == pool.asset_mint,
    )]
    pub asset_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"position", user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key(),
    )]
    pub user_position: Account<'info, UserPosition>,
    
    pub token_program: Program<'info, anchor_spl::token::Token>,
}

pub fn handler(ctx: Context<Borrow>, params: BorrowParams) -> Result<()> {
    let amount = params.amount;
    require!(amount > 0, OxygenError::InvalidParameter);
    
    let pool = &mut ctx.accounts.pool;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Update pool rates
    pool.update_rates(clock.unix_timestamp)?;
    
    // Check if pool has enough liquidity
    require!(
        pool.total_deposits.checked_sub(pool.total_borrows).unwrap_or(0) >= amount,
        OxygenError::InsufficientLiquidity
    );
    
    // Calculate borrow fee
    let fee_amount = (amount as u128)
        .checked_mul(pool.borrow_fee as u128)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(10000)
        .ok_or(ErrorCode::MathOverflow)? as u64;
    
    let borrow_amount_with_fee = amount
        .checked_add(fee_amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Simple mock for checking collateral sufficiency
    // In a real implementation, this would call into a collateral module
    // that checks prices from oracles and calculates cross-collateral values
    
    // Create a mock pool data map for health factor calculation
    // In a real implementation, this would involve fetching oracle prices 
    // and collateral parameters from all pools where the user has deposits
    let mut pool_data = HashMap::new();
    
    // Mock price and liquidation threshold - would come from oracle in real implementation
    // For simplicity, we'll use a 1:1 price and the pool's liquidation threshold
    pool_data.insert(pool.key(), (10000, pool.liquidation_threshold));
    
    // Add borrow to user position
    let scaled_amount = (borrow_amount_with_fee as u128)
        .checked_mul(1_000_000)  // Scale factor for precision
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(pool.cumulative_borrow_rate.checked_add(1).unwrap_or(1))
        .ok_or(ErrorCode::MathOverflow)?;
    
    // First add the borrow to position
    user_position.add_borrow(
        pool.key(), 
        borrow_amount_with_fee, 
        scaled_amount,
        pool.get_utilization_rate(),
    )?;
    
    // Then calculate health factor with the new borrow
    let health_factor = user_position.calculate_health_factor(&pool_data)?;
    
    // Check if health factor is still above minimum threshold
    const MIN_HEALTH_FACTOR: u64 = 10000; // 1.0 in scaled form
    require!(
        health_factor >= MIN_HEALTH_FACTOR,
        OxygenError::HealthFactorTooLow
    );
    
    // Update pool totals
    pool.total_borrows = pool.total_borrows
        .checked_add(borrow_amount_with_fee)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Transfer tokens from reserve to user
    let pool_seeds = &[
        b"pool".as_ref(),
        pool.asset_mint.as_ref(),
        &[pool.bump],
    ];
    
    let pool_signer = &[&pool_seeds[..]];
    
    let cpi_accounts = Transfer {
        from: ctx.accounts.asset_reserve.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.pool.to_account_info(),
    };
    
    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        pool_signer,
    );
    
    token::transfer(cpi_context, amount)?;
    
    user_position.last_updated = clock.unix_timestamp;
    
    msg!("Borrowed {} tokens from pool with {} fee", amount, fee_amount);
    
    Ok(())
}