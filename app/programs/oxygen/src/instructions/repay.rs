use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RepayParams {
    pub amount: u64,  // Amount to repay
}

#[derive(Accounts)]
pub struct Repay<'info> {
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

pub fn handler(ctx: Context<Repay>, params: RepayParams) -> Result<()> {
    let amount = params.amount;
    require!(amount > 0, OxygenError::InvalidParameter);
    
    let pool = &mut ctx.accounts.pool;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Update pool rates
    pool.update_rates(clock.unix_timestamp)?;
    
    // Find the borrow position
    let mut found_index = None;
    let mut current_borrowed_amount = 0;
    
    for (i, borrow) in user_position.borrows.iter().enumerate() {
        if borrow.pool == pool.key() {
            found_index = Some(i);
            current_borrowed_amount = borrow.amount_borrowed;
            break;
        }
    }
    
    require!(found_index.is_some(), OxygenError::BorrowNotFound);
    
    let borrow_index = found_index.unwrap();
    
    // Calculate actual repayable amount (can't repay more than owed)
    let repay_amount = std::cmp::min(amount, current_borrowed_amount);
    
    // Calculate how much borrow to remove (in scaled units)
    let borrow = &mut user_position.borrows[borrow_index];
    let scaled_amount_to_remove = (repay_amount as u128)
        .checked_mul(borrow.amount_scaled)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(borrow.amount_borrowed as u128)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Update borrow values
    borrow.amount_borrowed = borrow.amount_borrowed
        .checked_sub(repay_amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
    borrow.amount_scaled = borrow.amount_scaled
        .checked_sub(scaled_amount_to_remove)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Handle removal of the borrow entry if zero balance
    if borrow.amount_borrowed == 0 {
        user_position.borrows.remove(borrow_index);
    }
    
    // Update pool totals
    pool.total_borrows = pool.total_borrows
        .checked_sub(repay_amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
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
    
    token::transfer(cpi_context, repay_amount)?;
    
    // Update health factor
    // This is technically not necessary for repayments as they only improve health,
    // but it's good to keep the position's data accurate
    if !user_position.borrows.is_empty() {
        // Mock price data for simplistic health calculation
        // In a real implementation, this would involve fetching oracle prices
        let mut pool_data = std::collections::HashMap::new();
        
        // Mock price and liquidation threshold - would come from oracle in real implementation
        // For simplicity, we'll use a 1:1 price and the pool's liquidation threshold
        pool_data.insert(pool.key(), (10000, pool.liquidation_threshold));
        
        // Recalculate health factor
        let _ = user_position.calculate_health_factor(&pool_data)?;
    } else {
        // No borrows, so perfectly healthy
        user_position.health_factor = u64::MAX;
    }
    
    user_position.last_updated = clock.unix_timestamp;
    
    msg!("Repaid {} tokens to pool", repay_amount);
    
    Ok(())
}