use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawParams {
    pub amount: u64,  // Amount to withdraw
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

pub fn handler(ctx: Context<Withdraw>, params: WithdrawParams) -> Result<()> {
    let amount = params.amount;
    require!(amount > 0, OxygenError::InvalidParameter);
    
    let pool = &mut ctx.accounts.pool;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Update pool rates
    pool.update_rates(clock.unix_timestamp)?;
    
    // Find the collateral position
    let mut found_index = None;
    let mut current_deposited_amount = 0;
    
    for (i, collateral) in user_position.collaterals.iter().enumerate() {
        if collateral.pool == pool.key() {
            found_index = Some(i);
            current_deposited_amount = collateral.amount_deposited;
            break;
        }
    }
    
    require!(found_index.is_some(), OxygenError::CollateralNotFound);
    require!(current_deposited_amount >= amount, OxygenError::InsufficientBalance);
    
    let collateral_index = found_index.unwrap();
    
    // Calculate how much collateral to remove (in scaled units)
    let collateral = &mut user_position.collaterals[collateral_index];
    let scaled_amount_to_remove = (amount as u128)
        .checked_mul(collateral.amount_scaled)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(collateral.amount_deposited as u128)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Update collateral values
    collateral.amount_deposited = collateral.amount_deposited
        .checked_sub(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
    collateral.amount_scaled = collateral.amount_scaled
        .checked_sub(scaled_amount_to_remove)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Handle removal of the collateral entry if zero balance
    if collateral.amount_deposited == 0 {
        user_position.collaterals.remove(collateral_index);
    }
    
    // If the position has any borrows, verify the withdrawal doesn't break health factor
    if !user_position.borrows.is_empty() {
        // Create pool data map for health factor calculation
        // In a real implementation, this would involve fetching oracle prices
        let mut pool_data = HashMap::new();
        
        // Add the pool's liquidation threshold for calculation
        // For simplicity, we're using a 1:1 price ratio
        pool_data.insert(pool.key(), (10000, pool.liquidation_threshold));
        
        // Calculate health factor with the updated collateral
        let health_factor = user_position.calculate_health_factor(&pool_data)?;
        
        // Check if health factor is still above minimum threshold
        const MIN_HEALTH_FACTOR: u64 = 10000; // 1.0 in scaled form
        require!(
            health_factor >= MIN_HEALTH_FACTOR,
            OxygenError::HealthFactorTooLow
        );
    }
    
    // Update pool totals
    pool.total_deposits = pool.total_deposits
        .checked_sub(amount)
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
    
    msg!("Withdrawn {} tokens from pool", amount);
    
    Ok(())
}