use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;
use crate::modules::yield_generation::YieldModule;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ClaimYieldParams {
    pub reinvest: bool,        // Whether to reinvest yield back into the pool
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
    
    // Update pool rates and yields before claiming
    pool.update_rates(clock.unix_timestamp)?;
    
    // Check if the user has any lending position in this pool
    let mut has_lending_position = false;
    for collateral in &user_position.collaterals {
        if collateral.pool == pool.key() && collateral.is_lending {
            has_lending_position = true;
            break;
        }
    }
    
    require!(has_lending_position, OxygenError::CollateralNotFound);
    
    // Calculate accrued yield
    let accrued_yield = YieldModule::claim_yield(
        pool,
        user_position,
        &pool.key(),
        clock.unix_timestamp
    )?;
    
    require!(accrued_yield > 0, OxygenError::InvalidParameter);
    
    // Check if reinvestment is requested
    if params.reinvest {
        // If reinvesting, add to the user's collateral position
        for collateral in &mut user_position.collaterals {
            if collateral.pool == pool.key() && collateral.is_lending {
                // Add yield to the deposit
                collateral.amount_deposited = collateral.amount_deposited
                    .checked_add(accrued_yield)
                    .ok_or(ErrorCode::MathOverflow)?;
                
                // Update scaled amount to reflect the new deposit
                let additional_scaled = pool.deposit_to_scaled(accrued_yield)?;
                collateral.amount_scaled = collateral.amount_scaled
                    .checked_add(additional_scaled)
                    .ok_or(ErrorCode::MathOverflow)?;
                
                break;
            }
        }
        
        // Update pool totals to reflect the reinvestment
        pool.total_deposits = pool.total_deposits
            .checked_add(accrued_yield)
            .ok_or(ErrorCode::MathOverflow)?;
            
        if params.reinvest {
            // If reinvesting, also update the available lending supply
            pool.available_lending_supply = pool.available_lending_supply
                .checked_add(accrued_yield)
                .ok_or(ErrorCode::MathOverflow)?;
        }
        
        pool.update_utilization_rate()?;
        
        msg!("Reinvested yield of {} tokens", accrued_yield);
    } else {
        // If not reinvesting, transfer tokens to the user
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
        
        token::transfer(cpi_context, accrued_yield)?;
        
        msg!("Claimed yield of {} tokens", accrued_yield);
    }
    
    // Update user position's last updated timestamp
    user_position.last_updated = clock.unix_timestamp;
    
    Ok(())
}