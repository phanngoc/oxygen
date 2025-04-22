use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};
use crate::state::{Pool};
use crate::errors::OxygenError;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializePoolParams {
    pub optimal_utilization: u64,    // Optimal utilization rate (in basis points)
    pub loan_to_value: u64,          // Max loan-to-value ratio (in basis points)
    pub liquidation_threshold: u64,  // Liquidation threshold (in basis points)
    pub liquidation_bonus: u64,      // Liquidation bonus (in basis points)
    pub borrow_fee: u64,             // Fee for borrowing (in basis points)
    pub flash_loan_fee: u64,         // Fee for flash loans (in basis points)
    pub host_fee_percentage: u8,     // Host fee percentage (0-100)
    pub protocol_fee_percentage: u8, // Protocol fee percentage (0-100)
    pub lending_enabled: bool,       // Whether lending is enabled for this pool
    pub max_lending_ratio: u64,      // Maximum % of deposits that can be used for lending (basis points)
    pub min_lending_duration: u64,   // Minimum duration for lending positions in seconds
    pub lending_fee: u64,            // Fee for lending out assets (in basis points)
    pub lending_interest_share: u64, // Percentage of interest that goes to lenders (basis points)
    
    /// Ensures the pool cannot be upgraded after deployment
    pub immutable: bool,
    
    /// Set to true to make the pool completely admin-less
    pub admin_less: bool,
}

#[derive(Accounts)]
#[instruction(params: InitializePoolParams)]
pub struct InitializePool<'info> {
    /// This will be the user that initializes the pool
    /// When admin_less is true, this is purely to pay for the transaction
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = Pool::space(),
        seeds = [b"pool", asset_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,
    
    pub asset_mint: Account<'info, Mint>,
    
    #[account(
        init_if_needed,
        payer = authority,
        token::mint = asset_mint,
        token::authority = pool,
        seeds = [b"reserve", pool.key().as_ref()],
        bump
    )]
    pub asset_reserve: Account<'info, TokenAccount>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, anchor_spl::token::Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<InitializePool>, params: InitializePoolParams) -> Result<()> {
    // Validate parameters
    require!(
        params.loan_to_value <= 9000 && params.liquidation_threshold <= 9500,
        OxygenError::InvalidParameter
    );
    
    require!(
        params.loan_to_value < params.liquidation_threshold,
        OxygenError::InvalidParameter
    );
    
    require!(
        params.liquidation_bonus <= 2000,
        OxygenError::InvalidParameter
    );
    
    require!(
        params.host_fee_percentage + params.protocol_fee_percentage <= 100,
        OxygenError::InvalidParameter
    );

    // Validate new lending parameters
    require!(
        params.max_lending_ratio <= 10000, // Cannot exceed 100%
        OxygenError::InvalidParameter
    );

    require!(
        params.lending_fee <= 1000, // Max 10% fee
        OxygenError::InvalidParameter
    );

    require!(
        params.lending_interest_share <= 10000, // Max 100%
        OxygenError::InvalidParameter
    );
    
    // Enforce immutability if requested - this makes the pool non-upgradeable
    require!(
        params.immutable,
        OxygenError::PoolMustBeImmutable
    );

    // Enforce admin-less operation - no special admin privileges
    require!(
        params.admin_less,
        OxygenError::PoolMustBeAdminLess
    );
    
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;
    
    // Initialize pool
    pool.asset_mint = ctx.accounts.asset_mint.key();
    pool.asset_reserve = ctx.accounts.asset_reserve.key();
    pool.total_deposits = 0;
    pool.total_borrows = 0;
    pool.available_lending_supply = 0;
    pool.cumulative_borrow_rate = 1_000_000_000_000; // Initialize with 10^12 (1.0) for stable math
    pool.cumulative_lending_rate = 1_000_000_000_000; // Initialize with 10^12 (1.0) for stable math
    pool.last_updated = clock.unix_timestamp;
    pool.optimal_utilization = params.optimal_utilization;
    pool.loan_to_value = params.loan_to_value;
    pool.liquidation_threshold = params.liquidation_threshold;
    pool.liquidation_bonus = params.liquidation_bonus;
    pool.borrow_fee = params.borrow_fee;
    pool.flash_loan_fee = params.flash_loan_fee;
    pool.host_fee_percentage = params.host_fee_percentage;
    pool.protocol_fee_percentage = params.protocol_fee_percentage;
    
    // Initialize new lending parameters
    pool.lending_enabled = params.lending_enabled;
    pool.max_lending_ratio = params.max_lending_ratio;
    pool.min_lending_duration = params.min_lending_duration;
    pool.lending_fee = params.lending_fee;
    pool.lending_interest_share = params.lending_interest_share;
    pool.total_lent = 0; // Initialize total amount being lent out
    
    // Initialize ownership and immutability settings
    pool.user_deposits_authority = ctx.accounts.authority.key();
    pool.immutable = params.immutable;
    pool.admin_less = params.admin_less;
    
    pool.bump = *ctx.bumps.get("pool").unwrap();
    
    msg!("Initialized non-custodial lending pool for {} with immutable={}, admin_less={}", 
        pool.asset_mint,
        params.immutable,
        params.admin_less);
    
    Ok(())
}