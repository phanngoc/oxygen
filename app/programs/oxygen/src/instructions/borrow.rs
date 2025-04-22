use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;
use crate::events::{BorrowEvent, PoolUtilizationUpdatedEvent};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct BorrowParams {
    pub amount: u64,                  // Amount to borrow
    pub maintain_collateral_lending: bool, // Whether to maintain lending position while borrowing
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
    pub clock: Sysvar<'info, Clock>,
}

pub fn handler(ctx: Context<Borrow>, params: BorrowParams) -> Result<()> {
    let amount = params.amount;
    require!(amount > 0, OxygenError::InvalidParameter);
    
    let pool = &mut ctx.accounts.pool;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Update pool rates before any operations
    pool.update_rates(clock.unix_timestamp)?;
    
    // Check if the pool has enough liquidity
    require!(
        pool.total_deposits.checked_sub(pool.total_borrows).ok_or(ErrorCode::MathOverflow)? >= amount,
        OxygenError::InsufficientLiquidity
    );
    
    // Calculate maximum borrow amount based on user's collateral
    let mut has_sufficient_collateral = false;
    let mut user_has_collateral_for_asset = false;
    
    // Create pool data map for health factor calculation
    // In a real implementation, this would involve fetching oracle prices
    let mut pool_data = HashMap::new();
    pool_data.insert(pool.key(), (10000, pool.liquidation_threshold)); // Mock price data
    
    // Track if the user is already lending this asset to keep that status
    for collateral in &mut user_position.collaterals {
        if collateral.pool == pool.key() {
            user_has_collateral_for_asset = true;
            
            // Make sure we maintain lending status if the user asked for it
            if params.maintain_collateral_lending && collateral.is_lending {
                // We don't need to modify anything - the asset stays in lending pool
                msg!("Maintaining lending position while borrowing");
            }
        }
    }
    
    // Calculate borrowing capacity based on all user's collateral
    let (borrowing_capacity, _) = calculate_borrowing_capacity(user_position, &pool_data)?;
    
    // Get current borrow value in USD
    let current_borrow_value = calculate_borrow_value(user_position, &pool_data)?;
    
    // Check if user can borrow the requested amount
    let new_borrow_value = current_borrow_value.checked_add(amount as u128).ok_or(ErrorCode::MathOverflow)?;
    has_sufficient_collateral = new_borrow_value <= borrowing_capacity;
    
    require!(has_sufficient_collateral, OxygenError::InsufficientCollateral);
    
    // Calculate scaled borrow amount based on the cumulative borrow rate
    let scaled_borrow_amount = (amount as u128)
        .checked_mul(1_000_000_000_000) // 10^12 precision
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(pool.cumulative_borrow_rate)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Add to user's borrows
    user_position.add_borrow(
        pool.key(), 
        amount, 
        scaled_borrow_amount,
        pool.get_utilization_rate()  // Current interest rate
    )?;
    
    // Update pool totals
    pool.total_borrows = pool.total_borrows
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Recalculate pool utilization rate after borrow
    pool.update_utilization_rate()?;
    
    // Calculate health factor before the transfer
    let health_factor_before = user_position.calculate_health_factor(&pool_data)?;
    
    // Transfer tokens from pool reserve to user
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
    
    // Recalculate health factor after the borrow
    let health_factor_after = user_position.calculate_health_factor(&pool_data)?;
    user_position.last_updated = clock.unix_timestamp;
    
    // Emit borrow event
    emit!(BorrowEvent {
        user: ctx.accounts.user.key(),
        pool: pool.key(),
        asset_mint: pool.asset_mint,
        amount,
        interest_rate: pool.get_borrow_rate()?,
        timestamp: clock.unix_timestamp,
    });
    
    // Emit pool utilization updated event since borrowing changes utilization
    let utilization_rate = pool.get_utilization_rate();
    emit!(PoolUtilizationUpdatedEvent {
        pool: pool.key(),
        asset_mint: pool.asset_mint,
        utilization_rate,
        borrow_interest_rate: pool.get_borrow_rate()?,
        lending_interest_rate: pool.get_lending_rate()?,
        timestamp: clock.unix_timestamp,
    });
    
    msg!(
        "Borrowed {} tokens from pool. Health factor: {} -> {}",
        amount,
        health_factor_before,
        health_factor_after
    );
    
    Ok(())
}

/// Calculate the maximum borrowing capacity of a user based on their collateral
fn calculate_borrowing_capacity(
    user_position: &UserPosition,
    pool_data: &HashMap<Pubkey, (u64, u64)>
) -> Result<(u128, u128)> {
    let mut total_collateral_value = 0u128;
    let mut weighted_collateral_value = 0u128;
    
    // Calculate collateral value
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
            
            total_collateral_value = total_collateral_value
                .checked_add(value)
                .ok_or(ErrorCode::MathOverflow)?;
                
            weighted_collateral_value = weighted_collateral_value
                .checked_add(weighted_value)
                .ok_or(ErrorCode::MathOverflow)?;
        }
    }
    
    Ok((weighted_collateral_value, total_collateral_value))
}

/// Calculate the current borrow value in USD
fn calculate_borrow_value(
    user_position: &UserPosition,
    pool_data: &HashMap<Pubkey, (u64, u64)>
) -> Result<u128> {
    let mut total_borrowed_value = 0u128;
    
    // Calculate borrowed value
    for borrow in &user_position.borrows {
        if let Some((price, _)) = pool_data.get(&borrow.pool) {
            let value = (borrow.amount_borrowed as u128)
                .checked_mul(*price as u128)
                .ok_or(ErrorCode::MathOverflow)?;
            
            total_borrowed_value = total_borrowed_value
                .checked_add(value)
                .ok_or(ErrorCode::MathOverflow)?;
        }
    }
    
    Ok(total_borrowed_value)
}