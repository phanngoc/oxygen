use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositParams {
    pub amount: u64,             // Amount to deposit
    pub use_as_collateral: bool, // Whether to use deposit as collateral
}

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        init_if_needed,
        payer = user,
        space = UserPosition::space(),
        seeds = [b"position", user.key().as_ref()],
        bump,
    )]
    pub user_position: Account<'info, UserPosition>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, anchor_spl::token::Token>,
}

pub fn handler(ctx: Context<Deposit>, params: DepositParams) -> Result<()> {
    let amount = params.amount;
    require!(amount > 0, OxygenError::InvalidParameter);
    
    let pool = &mut ctx.accounts.pool;
    let user_position = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    
    // Update pool rates
    pool.update_rates(clock.unix_timestamp)?;
    
    // First time initialization of user position if new
    if user_position.owner == Pubkey::default() {
        user_position.owner = ctx.accounts.user.key();
        user_position.collaterals = Vec::new();
        user_position.borrows = Vec::new();
        user_position.health_factor = u64::MAX;
        user_position.last_updated = clock.unix_timestamp;
        user_position.bump = *ctx.bumps.get("user_position").unwrap();
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
    
    // Calculate scaled amount (for yield accounting)
    // If no deposits yet, start with 1:1 ratio
    let scaled_amount = if pool.total_deposits == 0 {
        amount as u128
    } else {
        (amount as u128)
            .checked_mul(pool.total_deposits as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div((pool.total_deposits) as u128)
            .ok_or(ErrorCode::MathOverflow)?
    };
    
    // Add collateral to user position
    user_position.add_collateral(
        pool.key(), 
        amount, 
        scaled_amount,
    )?;
    
    // Update pool totals
    pool.total_deposits = pool.total_deposits
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
        
    user_position.last_updated = clock.unix_timestamp;
    
    msg!("Deposited {} tokens into pool", amount);
    
    Ok(())
}