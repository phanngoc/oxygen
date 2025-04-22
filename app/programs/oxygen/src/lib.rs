use anchor_lang::prelude::*;

pub mod state;
pub mod instructions;
pub mod modules;
pub mod errors;

use instructions::*;

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

    /// Place a leveraged trade using Serum DEX
    pub fn trade_with_leverage(ctx: Context<TradeWithLeverage>, params: TradeParams) -> Result<()> {
        instructions::trade::handler(ctx, params)
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