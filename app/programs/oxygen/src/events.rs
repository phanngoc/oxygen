use anchor_lang::prelude::*;

// Deposit Events
#[event]
pub struct DepositEvent {
    pub user: Pubkey,             // User who made the deposit
    pub pool: Pubkey,             // Pool where deposit was made
    pub asset_mint: Pubkey,       // Asset that was deposited
    pub amount: u64,              // Amount deposited
    pub is_collateral: bool,      // Whether deposit is used as collateral
    pub is_lending: bool,         // Whether deposit is used for lending
    pub timestamp: i64,           // When the deposit happened
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,             // User who made the withdrawal
    pub pool: Pubkey,             // Pool where withdrawal was made
    pub asset_mint: Pubkey,       // Asset that was withdrawn
    pub amount: u64,              // Amount withdrawn
    pub from_collateral: bool,    // Whether withdrawn from collateral
    pub from_lending: bool,       // Whether withdrawn from lending
    pub timestamp: i64,           // When the withdrawal happened
}

// Lending specific events
#[event]
pub struct LendingEnabledEvent {
    pub user: Pubkey,             // User who enabled lending
    pub pool: Pubkey,             // Pool where lending was enabled
    pub asset_mint: Pubkey,       // Asset that was enabled for lending
    pub amount: u64,              // Amount enabled for lending
    pub timestamp: i64,           // When lending was enabled
}

#[event]
pub struct LendingDisabledEvent {
    pub user: Pubkey,             // User who disabled lending
    pub pool: Pubkey,             // Pool where lending was disabled
    pub asset_mint: Pubkey,       // Asset that was disabled for lending
    pub amount: u64,              // Amount disabled from lending
    pub timestamp: i64,           // When lending was disabled
}

// Borrow Events
#[event]
pub struct BorrowEvent {
    pub user: Pubkey,             // User who borrowed
    pub pool: Pubkey,             // Pool borrowed from
    pub asset_mint: Pubkey,       // Asset that was borrowed
    pub amount: u64,              // Amount borrowed
    pub interest_rate: u64,       // Interest rate at time of borrow
    pub timestamp: i64,           // When the borrow happened
}

#[event]
pub struct RepayEvent {
    pub user: Pubkey,             // User who repaid
    pub pool: Pubkey,             // Pool repaid to
    pub asset_mint: Pubkey,       // Asset that was repaid
    pub amount: u64,              // Amount repaid
    pub interest_paid: u64,       // Interest portion of payment
    pub principal_paid: u64,      // Principal portion of payment
    pub timestamp: i64,           // When the repay happened
}

// Liquidation event
#[event]
pub struct LiquidationEvent {
    pub liquidator: Pubkey,       // User who performed the liquidation
    pub liquidated: Pubkey,       // User who was liquidated
    pub pool: Pubkey,             // Pool where liquidation occurred
    pub asset_mint: Pubkey,       // Asset that was liquidated
    pub collateral_amount: u64,   // Amount of collateral liquidated
    pub debt_amount: u64,         // Amount of debt repaid
    pub liquidation_bonus: u64,   // Bonus received by liquidator
    pub timestamp: i64,           // When the liquidation happened
}

// Yield events
#[event]
pub struct YieldAccruedEvent {
    pub pool: Pubkey,             // Pool where yield accrued
    pub asset_mint: Pubkey,       // Asset that generated yield
    pub yield_amount: u64,        // Amount of yield generated
    pub lending_rate: u128,       // Lending rate at time of accrual
    pub timestamp: i64,           // When the yield accrued
}

#[event]
pub struct YieldClaimedEvent {
    pub user: Pubkey,             // User who claimed yield
    pub pool: Pubkey,             // Pool where yield was claimed from
    pub asset_mint: Pubkey,       // Asset that generated yield
    pub amount: u64,              // Amount of yield claimed
    pub timestamp: i64,           // When the yield was claimed
}

// Pool events
#[event]
pub struct PoolUtilizationUpdatedEvent {
    pub pool: Pubkey,             // Pool that was updated
    pub asset_mint: Pubkey,       // Asset in the pool
    pub utilization_rate: u64,    // New utilization rate
    pub borrow_interest_rate: u64, // New borrow interest rate
    pub lending_interest_rate: u64, // New lending interest rate
    pub timestamp: i64,           // When the update happened
}