use anchor_lang::prelude::*;

pub mod state;
pub mod instructions;
pub mod modules;
pub mod errors;

use instructions::*;
use std::collections::HashMap;

declare_id!("Oxygen111111111111111111111111111111111111111");

#[program]
pub mod oxygen {
    use super::*;

    /// Initialize a new lending pool for a specific asset
    pub fn initialize_pool(ctx: Context<InitializePool>, params: InitializePoolParams) -> Result<()> {
        instructions::init_pool::handler(ctx, params)
    }

    /// Deposit tokens into a lending pool
    pub fn deposit(ctx: Context<Deposit>, params: DepositParams) -> Result<()> {
        instructions::deposit::handler(ctx, params)
    }

    /// Withdraw tokens from a lending pool
    pub fn withdraw(ctx: Context<Withdraw>, params: WithdrawParams) -> Result<()> {
        instructions::withdraw::handler(ctx, params)
    }

    /// Borrow tokens from a lending pool using cross-collateralization
    pub fn borrow(ctx: Context<Borrow>, params: BorrowParams) -> Result<()> {
        instructions::borrow::handler(ctx, params)
    }

    /// Repay borrowed tokens to a lending pool
    pub fn repay(ctx: Context<Repay>, params: RepayParams) -> Result<()> {
        instructions::repay::handler(ctx, params)
    }

    /// Open a leveraged trade position using Serum DEX
    pub fn open_trade(ctx: Context<TradeWithLeverage>, params: TradeParams) -> Result<()> {
        instructions::trade::open_trade(ctx, params)
    }
    
    /// Close an existing leveraged trade position
    pub fn close_trade(ctx: Context<CloseTradePosition>, params: ClosePositionParams) -> Result<()> {
        instructions::trade::close_position(ctx, params)
    }
    
    /// Monitor and liquidate positions if necessary
    pub fn monitor_positions(ctx: Context<CloseTradePosition>, current_prices: HashMap<Pubkey, u64>) -> Result<()> {
        instructions::trade::monitor_positions_for_liquidation(ctx, current_prices)
    }
    
    /// Process funding rates for open leveraged positions
    pub fn process_funding(ctx: Context<CloseTradePosition>, funding_rates: HashMap<Pubkey, i64>) -> Result<()> {
        instructions::trade::process_funding_rates(ctx, funding_rates)
    }
    
    /// Get user's open leveraged positions
    pub fn get_open_positions(ctx: Context<CloseTradePosition>) -> Result<Vec<u64>> {
        instructions::trade::get_open_positions(ctx)
    }

    /// Liquidate an undercollateralized position
    pub fn liquidate(ctx: Context<Liquidate>, params: LiquidateParams) -> Result<()> {
        instructions::liquidate::handler(ctx, params)
    }

    /// Claim yield generated from lending
    pub fn claim_yield(ctx: Context<ClaimYield>, params: ClaimYieldParams) -> Result<()> {
        instructions::claim_yield::handler(ctx, params)
    }
}