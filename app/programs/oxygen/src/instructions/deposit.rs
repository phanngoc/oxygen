use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;
use crate::modules::yield_generation::YieldModule;
use crate::events::{DepositEvent, LendingEnabledEvent, PoolUtilizationUpdatedEvent};

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
    
    // Ensure the pool is non-custodial and immutable
    require!(pool.immutable, OxygenError::PoolIsUpgradable);
    require!(pool.admin_less, OxygenError::AdminOperationsNotSupported);
    
    // Verify that operations are not paused - should never be possible in admin_less mode
    if pool.operation_state_flags & 0x1 != 0 {
        return Err(OxygenError::OperationPaused.into());
    }
    
    // Strictly enforce user signature - only users can move their funds
    require!(
        ctx.accounts.user.is_signer,
        OxygenError::UserSignatureRequired
    );
    
    // Ensure position belongs to the current user
    require!(
        user_position.owner == ctx.accounts.user.key(),
        OxygenError::OnlyPositionOwnerAllowed
    );
    
    // Check if lending is enabled on the pool when the user wants to enable lending
    if params.enable_lending && !pool.lending_enabled {
        return Err(OxygenError::LendingNotEnabled.into());
    }
    
    // Check if there's a rate limit on position modifications
    if clock.unix_timestamp - user_position.last_updated < 10 { // 10 second cooldown
        return Err(OxygenError::PositionModificationCooldown.into());
    }
    
    // Check if the user has enough token balance
    let user_token_balance = ctx.accounts.user_token_account.amount;
    if user_token_balance < amount {
        return Err(OxygenError::InsufficientBalance.into());
    }
    
    // Check transaction size limits
    const MAX_DEPOSIT_SIZE: u64 = 1_000_000_000_000; // Example: 1 trillion token units
    if amount > MAX_DEPOSIT_SIZE {
        return Err(OxygenError::TransactionSizeExceeded.into());
    }
    
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
            
            // Set lending status and timestamp
            collateral.is_lending = params.enable_lending;
            collateral.deposit_timestamp = clock.unix_timestamp;
            break;
        }
    }
    
    // Check lending capacity when enabling lending
    if params.enable_lending {
        // Calculate how much is already being lent out
        let total_after_deposit = pool.total_lent.checked_add(amount)
            .ok_or(OxygenError::MathOverflow)?;
            
        // Calculate the maximum lending capacity based on the max_lending_ratio
        let max_lending_capacity = (pool.total_deposits as u128)
            .checked_mul(pool.max_lending_ratio as u128)
            .ok_or(OxygenError::MathOverflow)?
            .checked_div(10000)
            .ok_or(OxygenError::MathOverflow)? as u64;
            
        // Ensure we don't exceed the maximum lending capacity
        if total_after_deposit > max_lending_capacity {
            return Err(OxygenError::MaxLendingCapacityReached.into());
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
        .ok_or(OxygenError::MathOverflow)?;
    
    // If lending is enabled, update the lending supply and total_lent
    if params.enable_lending {
        pool.available_lending_supply = pool.available_lending_supply
            .checked_add(amount)
            .ok_or(OxygenError::MathOverflow)?;
        
        pool.total_lent = pool.total_lent
            .checked_add(amount)
            .ok_or(OxygenError::MathOverflow)?;
    }
    
    // Recalculate pool utilization rate after deposit
    pool.update_utilization_rate()?;
    
    // Update health factor using oracle prices if available
    let mut pool_data = HashMap::new();
    
    if pool.price_oracle != Pubkey::default() {
        // Using oracle price for calculations
        if (!verify_oracle_freshness(pool)) {
            return Err(OxygenError::StaleOracleData.into());
        }
        
        pool_data.insert(pool.key(), (pool.last_oracle_price, pool.liquidation_threshold));
    } else {
        // Fallback to default pricing
        pool_data.insert(pool.key(), (10000, pool.liquidation_threshold));
    }
    
    let _ = user_position.calculate_health_factor(&pool_data)?;
    
    user_position.last_updated = clock.unix_timestamp;
    
    // Emit deposit event
    emit!(DepositEvent {
        user: ctx.accounts.user.key(),
        pool: pool.key(),
        asset_mint: pool.asset_mint,
        amount,
        is_collateral: params.use_as_collateral,
        is_lending: params.enable_lending,
        timestamp: clock.unix_timestamp,
    });
    
    // If lending is enabled, also emit a lending enabled event
    if params.enable_lending {
        emit!(LendingEnabledEvent {
            user: ctx.accounts.user.key(),
            pool: pool.key(),
            asset_mint: pool.asset_mint,
            amount,
            timestamp: clock.unix_timestamp,
        });
    }
    
    // Emit pool utilization updated event
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
        "Deposited {} tokens to pool (collateral: {}, lending: {})",
        amount,
        params.use_as_collateral,
        params.enable_lending
    );
    
    Ok(())
}

// Helper function to verify oracle price freshness
fn verify_oracle_freshness(pool: &Pool) -> bool {
    if pool.price_oracle == Pubkey::default() {
        return false;
    }
    
    // Check if the oracle price update is within an acceptable time window
    let max_oracle_staleness = 300; // 5 minutes in seconds
    let clock = Clock::get().unwrap();
    
    clock.unix_timestamp - pool.last_oracle_update < max_oracle_staleness
}