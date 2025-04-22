use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ClaimYieldParams {
    pub reinvest: bool,  // Whether to reinvest yield back into the pool
}

#[derive(Accounts)]
pub struct ClaimYield<'info> {
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

pub fn handler(ctx: Context<ClaimYield>, params: ClaimYieldParams) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Update pool rates to ensure all yield has been accrued
    pool.update_rates(clock.unix_timestamp)?;
    
    // Find user's collateral in the specified pool
    let mut collateral_position_idx = None;
    for (i, collateral) in user_position.collaterals.iter().enumerate() {
        if collateral.pool == pool.key() {
            collateral_position_idx = Some(i);
            break;
        }
    }
    
    let collateral_position_idx = collateral_position_idx.ok_or(OxygenError::InvalidParameter)?;
    let collateral_position = &mut user_position.collaterals[collateral_position_idx];
    
    // Calculate accrued yield by comparing scaled amount to current value
    // In a lending pool, the exchange rate between scaled units and tokens increases over time
    // as interest accrues
    
    // Calculate the current token amount based on scaled amount
    let current_token_value = if pool.total_deposits == 0 {
        collateral_position.amount_scaled as u64
    } else {
        ((collateral_position.amount_scaled as u128)
            .checked_mul(pool.total_deposits as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000) // Assuming a scale factor for precision
            .ok_or(ErrorCode::MathOverflow)?) as u64
    };
    
    let yield_amount = current_token_value
        .checked_sub(collateral_position.amount_deposited)
        .unwrap_or(0);
        
    require!(yield_amount > 0, OxygenError::InvalidParameter);
    
    // Transfer yield tokens from reserve to user
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
    
    token::transfer(cpi_context, yield_amount)?;
    
    if params.reinvest {
        // If reinvesting, add the yield amount back to collateral
        collateral_position.amount_deposited = collateral_position.amount_deposited
            .checked_add(yield_amount)
            .ok_or(ErrorCode::MathOverflow)?;
            
        // Update scaled amount as well
        // In practice this would involve a more complex calculation based on current exchange rate
        let additional_scaled_amount = (yield_amount as u128)
            .checked_mul(10000) // Assuming a scale factor for precision
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(pool.total_deposits as u128)
            .ok_or(ErrorCode::MathOverflow)?;
            
        collateral_position.amount_scaled = collateral_position.amount_scaled
            .checked_add(additional_scaled_amount)
            .ok_or(ErrorCode::MathOverflow)?;
            
        msg!("Claimed and reinvested {} yield tokens", yield_amount);
    } else {
        // If not reinvesting, adjust the user's position to reflect claimed yield
        collateral_position.amount_deposited = current_token_value;
        
        msg!("Claimed {} yield tokens", yield_amount);
    }
    
    // Update last updated timestamp
    user_position.last_updated = clock.unix_timestamp;
    
    Ok(())
}