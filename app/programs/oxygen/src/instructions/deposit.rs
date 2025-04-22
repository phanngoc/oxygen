use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;
use crate::modules::yield_generation::YieldModule;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositParams {
    pub amount: u64,                  // Amount to deposit
    pub use_as_collateral: bool,      // Whether to use as collateral
    pub enable_lending: bool,         // Whether to enable lending to other users
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)
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
    pub clock: Sysvar<'info, Clock>,
}

pub fn handler(ctx: Context<Deposit>, params: DepositParams) -> Result<()> {
    let amount = params.amount;
    require!(amount > 0, OxygenError::InvalidParameter);
    
    let pool = &mut ctx.accounts.pool;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Update pool rates before any operations
    pool.update_rates(clock.unix_timestamp)?;
    
    // Calculate scaled amount based on the current exchange rate
    // This accounts for accumulated yield in the pool
    let scaled_amount = pool.deposit_to_scaled(amount)?;
    
    // Add deposit to user's collateral position
    user_position.add_collateral(
        pool.key(),
        amount,
        scaled_amount
    )?;
    
    // Set the collateral usage flag for this deposit
    // Find the collateral we just added/updated
    for collateral in &mut user_position.collaterals {
        if collateral.pool == pool.key() {
            collateral.is_collateral = params.use_as_collateral;
            
            // Set lending status in a separate field we'll add to the CollateralPosition struct
            collateral.is_lending = params.enable_lending;
            break;
        }
    }
    
    // Transfer tokens from user to pool reserve
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.asset_reserve.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    
    let cpi_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );
    
    token::transfer(cpi_context, amount)?;
    
    // Update pool totals
    pool.total_deposits = pool.total_deposits
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // If lending is enabled, update the lending supply
    if params.enable_lending {
        pool.available_lending_supply = pool.available_lending_supply
            .checked_add(amount)
            .ok_or(ErrorCode::MathOverflow)?;
    }
    
    // Recalculate pool utilization rate after deposit
    pool.update_utilization_rate()?;
    
    // Update health factor
    let mut pool_data = HashMap::new();
    pool_data.insert(pool.key(), (10000, pool.liquidation_threshold)); // Mock price data
    let _ = user_position.calculate_health_factor(&pool_data)?;
    
    user_position.last_updated = clock.unix_timestamp;
    
    msg!(
        "Deposited {} tokens to pool (collateral: {}, lending: {})",
        amount,
        params.use_as_collateral,
        params.enable_lending
    );
    
    Ok(())
}