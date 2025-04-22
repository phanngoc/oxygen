use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use std::collections::HashMap;
use crate::state::{Pool, UserPosition};
use crate::errors::OxygenError;
use crate::events::{WithdrawEvent, LendingDisabledEvent, PoolUtilizationUpdatedEvent};
// Import the wallet integration module
use crate::modules::wallet_integration::WalletIntegration;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawParams {
    pub amount: u64,  // Amount to withdraw
    pub is_lending_withdrawal: bool, // Flag to indicate if this is a lending position withdrawal
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
    
    // NON-CUSTODIAL: Ensure the pool is immutable and admin-less
    require!(pool.immutable, OxygenError::PoolIsUpgradable);
    require!(pool.admin_less, OxygenError::AdminOperationsNotSupported);
    
    // NON-CUSTODIAL: Validate that the user is signing their own withdrawal
    WalletIntegration::validate_owner_signed(
        &user_position.owner,
        &ctx.accounts.user
    )?;
    
    // Check if operations are currently paused - should never happen in admin-less mode
    if pool.operation_state_flags & 0x1 != 0 {
        return Err(OxygenError::OperationPaused.into());
    }
    
    // For lending withdrawals, verify lending is enabled for this pool
    if params.is_lending_withdrawal && !pool.lending_enabled {
        return Err(OxygenError::LendingNotEnabled.into());
    }
    
    // Check for rate limiting - prevent frequent position modifications
    if clock.unix_timestamp - user_position.last_updated < 10 { // 10 second cooldown
        return Err(OxygenError::PositionModificationCooldown.into());
    }
    
    // Update pool rates
    pool.update_rates(clock.unix_timestamp)?;
    
    // Find the collateral position
    let mut found_index = None;
    let mut current_deposited_amount = 0;
    let mut position_start_timestamp = 0;
    
    for (i, collateral) in user_position.collaterals.iter().enumerate() {
        if collateral.pool == pool.key() {
            // For lending withdrawals, ensure the position is marked as lending
            if params.is_lending_withdrawal && !collateral.is_lending {
                continue;
            }
            
            // For collateral withdrawals, ensure the position is marked as collateral
            if !params.is_lending_withdrawal && !collateral.is_collateral {
                continue;
            }
            
            found_index = Some(i);
            current_deposited_amount = collateral.amount_deposited;
            position_start_timestamp = collateral.deposit_timestamp;
            break;
        }
    }
    
    require!(found_index.is_some(), OxygenError::CollateralNotFound);
    require!(current_deposited_amount >= amount, OxygenError::InsufficientBalance);
    
    // Check for minimum lending duration if this is a lending withdrawal
    if params.is_lending_withdrawal && 
       pool.min_lending_duration > 0 &&
       clock.unix_timestamp - position_start_timestamp < pool.min_lending_duration as i64 {
        return Err(OxygenError::MinLendingDurationNotMet.into());
    }
    
    let collateral_index = found_index.unwrap();
    
    // Calculate how much collateral to remove (in scaled units)
    let collateral = &mut user_position.collaterals[collateral_index];
    
    // Guard against divide-by-zero
    if collateral.amount_deposited == 0 {
        return Err(OxygenError::MathOverflow.into());
    }
    
    let scaled_amount_to_remove = (amount as u128)
        .checked_mul(collateral.amount_scaled)
        .ok_or(OxygenError::MathOverflow)?
        .checked_div(collateral.amount_deposited as u128)
        .ok_or(OxygenError::MathOverflow)?;
    
    // Update collateral values
    collateral.amount_deposited = collateral.amount_deposited
        .checked_sub(amount)
        .ok_or(OxygenError::MathOverflow)?;
    
    collateral.amount_scaled = collateral.amount_scaled
        .checked_sub(scaled_amount_to_remove)
        .ok_or(OxygenError::MathOverflow)?;
    
    // If lending withdrawal, check if we need to update the is_lending flag
    if params.is_lending_withdrawal && collateral.amount_deposited == 0 {
        collateral.is_lending = false;
    }
    
    // If collateral withdrawal, check if we need to update the is_collateral flag
    if !params.is_lending_withdrawal && collateral.amount_deposited == 0 {
        collateral.is_collateral = false;
    }
    
    // Handle removal of the collateral entry if zero balance and neither lending nor collateral
    if collateral.amount_deposited == 0 && !collateral.is_lending && !collateral.is_collateral {
        user_position.collaterals.remove(collateral_index);
    }
    
    // If the position has any borrows and this is a collateral withdrawal, verify the withdrawal doesn't break health factor
    if !params.is_lending_withdrawal && !user_position.borrows.is_empty() {
        // Create pool data map for health factor calculation
        let mut pool_data = HashMap::new();
        
        // Check if we should use oracle prices
        if pool.price_oracle != Pubkey::default() {
            // In a real implementation, fetch the oracle price
            // Here we're just using a placeholder implementation
            if !verify_oracle_freshness(pool) {
                return Err(OxygenError::StaleOracleData.into());
            }
            
            // Add the pool with oracle price and liquidation threshold
            pool_data.insert(pool.key(), (pool.last_oracle_price, pool.liquidation_threshold));
        } else {
            // Fallback to a 1:1 price ratio
            pool_data.insert(pool.key(), (10000, pool.liquidation_threshold));
        }
        
        // Calculate health factor with the updated collateral
        let health_factor = user_position.calculate_health_factor(&pool_data)?;
        
        // Check if health factor is still above minimum threshold
        const MIN_HEALTH_FACTOR: u64 = 10000; // 1.0 in scaled form
        require!(
            health_factor >= MIN_HEALTH_FACTOR,
            OxygenError::HealthFactorTooLow
        );
    }
    
    // If this is a lending withdrawal, perform additional checks
    if params.is_lending_withdrawal {
        // The available liquidity is the total deposits minus the total borrows
        let available_liquidity = pool.total_deposits
            .checked_sub(pool.total_borrows)
            .ok_or(OxygenError::MathOverflow)?;
            
        require!(
            available_liquidity >= amount,
            OxygenError::InsufficientLiquidity
        );
        
        // Check if there are enough reserves to cover the withdrawal
        let reserve_balance = ctx.accounts.asset_reserve.amount;
        if reserve_balance < amount {
            return Err(OxygenError::InsufficientReserves.into());
        }
        
        // Check if utilization is too high for withdrawal
        let utilization = pool.get_utilization_rate();
        const MAX_UTILIZATION_FOR_WITHDRAWAL: u64 = 9500; // 95%
        
        if utilization > MAX_UTILIZATION_FOR_WITHDRAWAL {
            return Err(OxygenError::UtilizationTooHigh.into());
        }
    }
    
    // Update pool totals
    if params.is_lending_withdrawal {
        // For lending withdrawals, update the lending pool metrics
        pool.total_lent = pool.total_lent
            .checked_sub(amount)
            .ok_or(OxygenError::MathOverflow)?;
    } else {
        // For regular withdrawals
        pool.total_deposits = pool.total_deposits
            .checked_sub(amount)
            .ok_or(OxygenError::MathOverflow)?;
    }
    
    // Transfer tokens from reserve to user
    let pool_seeds = &[
        b"pool".as_ref(),
        pool.asset_mint.as_ref(),
        &[pool.bump],
    ];
    
    let pool_signer = &[&pool_seeds[..]];
    
    // NON-CUSTODIAL: Generate transaction metadata for wallet transparency
    let transaction_metadata = WalletIntegration::get_transaction_metadata(
        &[amount.to_le_bytes().as_ref(), b"withdraw"].concat()
    )?;
    
    // NON-CUSTODIAL: Ensure no admin operations are included in this transaction
    WalletIntegration::validate_no_admin_operations(
        &[0u8, 0u8, 0u8, 0u8] // Placeholder for actual instruction data
    )?;
    
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
    
    // Emit withdraw event with appropriate flags based on the withdrawal type
    emit!(WithdrawEvent {
        user: ctx.accounts.user.key(),
        pool: pool.key(),
        asset_mint: pool.asset_mint,
        amount,
        from_collateral: !params.is_lending_withdrawal,
        from_lending: params.is_lending_withdrawal,
        timestamp: clock.unix_timestamp,
    });
    
    // If this is a lending withdrawal, also emit a lending disabled event
    if params.is_lending_withdrawal {
        emit!(LendingDisabledEvent {
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
    
    // Emit event based on withdrawal type
    if params.is_lending_withdrawal {
        msg!("Withdrawn {} tokens from lending position", amount);
    } else {
        msg!("Withdrawn {} tokens from collateral position", amount);
    }
    
    Ok(())
}

// Helper function to verify oracle price freshness
fn verify_oracle_freshness(pool: &Pool) -> bool {
    if pool.price_oracle == Pubkey::default() {
        return false;
    }
    
    // In a production implementation, this would check if the oracle
    // price update is within an acceptable time window
    let max_oracle_staleness = 300; // 5 minutes in seconds
    let clock = Clock::get().unwrap();
    
    clock.unix_timestamp - pool.last_oracle_update < max_oracle_staleness
}