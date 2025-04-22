use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct LiquidateParams {
    pub amount: u64,                 // Amount of debt to liquidate
    pub receive_collateral_asset: bool, // Whether to receive collateral token or the equivalent in another asset
}

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,
    
    pub user: AccountInfo<'info>,
    
    #[account(
        mut,
        seeds = [b"pool", debt_pool.asset_mint.as_ref()],
        bump = debt_pool.bump,
    )]
    pub debt_pool: Account<'info, Pool>,
    
    #[account(
        mut,
        seeds = [b"pool", collateral_pool.asset_mint.as_ref()],
        bump = collateral_pool.bump,
    )]
    pub collateral_pool: Account<'info, Pool>,
    
    #[account(
        mut,
        constraint = debt_reserve.mint == debt_pool.asset_mint,
    )]
    pub debt_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = collateral_reserve.mint == collateral_pool.asset_mint,
    )]
    pub collateral_reserve: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = liquidator_debt_token_account.mint == debt_pool.asset_mint,
        constraint = liquidator_debt_token_account.owner == liquidator.key(),
    )]
    pub liquidator_debt_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = liquidator_collateral_token_account.mint == collateral_pool.asset_mint,
        constraint = liquidator_collateral_token_account.owner == liquidator.key(),
    )]
    pub liquidator_collateral_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"position", user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key(),
    )]
    pub user_position: Account<'info, UserPosition>,
    
    pub token_program: Program<'info, anchor_spl::token::Token>,
}

pub fn handler(ctx: Context<Liquidate>, params: LiquidateParams) -> Result<()> {
    require!(params.amount > 0, OxygenError::InvalidParameter);
    
    let debt_pool = &mut ctx.accounts.debt_pool;
    let collateral_pool = &mut ctx.accounts.collateral_pool;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Update pool rates
    debt_pool.update_rates(clock.unix_timestamp)?;
    collateral_pool.update_rates(clock.unix_timestamp)?;
    
    // Create a mock pool data map for health factor calculation
    // In a real implementation, this would involve fetching oracle prices and parameters
    let mut pool_data = HashMap::new();
    
    // Mock prices - would come from oracle in real implementation
    pool_data.insert(debt_pool.key(), (10000, debt_pool.liquidation_threshold));
    pool_data.insert(collateral_pool.key(), (10000, collateral_pool.liquidation_threshold));
    
    // Calculate current health factor
    user_position.calculate_health_factor(&pool_data)?;
    
    // Check if position is eligible for liquidation
    const LIQUIDATION_THRESHOLD: u64 = 10000; // 1.0 in scaled form
    require!(
        user_position.health_factor < LIQUIDATION_THRESHOLD,
        OxygenError::CannotLiquidate
    );
    
    // Find user's debt in the specified pool
    let mut debt_position_idx = None;
    for (i, borrow) in user_position.borrows.iter().enumerate() {
        if borrow.pool == debt_pool.key() {
            debt_position_idx = Some(i);
            break;
        }
    }
    
    let debt_position_idx = debt_position_idx.ok_or(OxygenError::InvalidParameter)?;
    let debt_position = &mut user_position.borrows[debt_position_idx];
    
    // Check if liquidation amount <= borrow amount
    require!(
        params.amount <= debt_position.amount_borrowed,
        OxygenError::InvalidParameter
    );
    
    // Find user's collateral in the specified pool
    let mut collateral_position_idx = None;
    for (i, collateral) in user_position.collaterals.iter().enumerate() {
        if collateral.pool == collateral_pool.key() {
            collateral_position_idx = Some(i);
            break;
        }
    }
    
    let collateral_position_idx = collateral_position_idx.ok_or(OxygenError::InvalidParameter)?;
    let collateral_position = &mut user_position.collaterals[collateral_position_idx];
    
    // Calculate liquidation bonus (e.g., 5-10%)
    let bonus_rate = debt_pool.liquidation_bonus;
    
    // Calculate collateral value to seize including bonus
    // In a real implementation, this would use asset-specific prices from oracles
    let collateral_to_seize = (params.amount as u128)
        .checked_mul(10000 + bonus_rate as u128)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(10000)
        .ok_or(ErrorCode::MathOverflow)? as u64;
    
    // Ensure user has enough collateral
    require!(
        collateral_position.amount_deposited >= collateral_to_seize,
        OxygenError::InsufficientCollateral
    );
    
    // Transfer debt tokens from liquidator to reserve
    let cpi_accounts = Transfer {
        from: ctx.accounts.liquidator_debt_token_account.to_account_info(),
        to: ctx.accounts.debt_reserve.to_account_info(),
        authority: ctx.accounts.liquidator.to_account_info(),
    };
    
    let cpi_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );
    
    token::transfer(cpi_context, params.amount)?;
    
    // Transfer collateral tokens from reserve to liquidator
    let pool_seeds = &[
        b"pool".as_ref(),
        collateral_pool.asset_mint.as_ref(),
        &[collateral_pool.bump],
    ];
    
    let pool_signer = &[&pool_seeds[..]];
    
    let cpi_accounts = Transfer {
        from: ctx.accounts.collateral_reserve.to_account_info(),
        to: ctx.accounts.liquidator_collateral_token_account.to_account_info(),
        authority: ctx.accounts.collateral_pool.to_account_info(),
    };
    
    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        pool_signer,
    );
    
    token::transfer(cpi_context, collateral_to_seize)?;
    
    // Update user's debt position
    debt_position.amount_borrowed = debt_position.amount_borrowed
        .checked_sub(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;
        
    if debt_position.amount_borrowed == 0 {
        // Remove empty debt position
        user_position.borrows.remove(debt_position_idx);
    }
    
    // Update user's collateral position
    collateral_position.amount_deposited = collateral_position.amount_deposited
        .checked_sub(collateral_to_seize)
        .ok_or(ErrorCode::MathOverflow)?;
        
    if collateral_position.amount_deposited == 0 {
        // Remove empty collateral position
        user_position.collaterals.remove(collateral_position_idx);
    }
    
    // Update pool totals
    debt_pool.total_borrows = debt_pool.total_borrows
        .checked_sub(params.amount)
        .ok_or(ErrorCode::MathOverflow)?;
        
    collateral_pool.total_deposits = collateral_pool.total_deposits
        .checked_sub(collateral_to_seize)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // Recalculate health factor after liquidation
    user_position.calculate_health_factor(&pool_data)?;
    user_position.last_updated = clock.unix_timestamp;
    
    msg!("Liquidated {} debt tokens for {} collateral tokens", 
        params.amount, 
        collateral_to_seize
    );
    
    Ok(())
}