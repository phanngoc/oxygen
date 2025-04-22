use anchor_lang::prelude::*;

#[error_code]
pub enum OxygenError {
    #[msg("Math operation overflow")]
    MathOverflow,
    
    #[msg("Insufficient liquidity in pool")]
    InsufficientLiquidity,
    
    #[msg("Insufficient collateral for this action")]
    InsufficientCollateral,
    
    #[msg("Health factor below minimum threshold")]
    HealthFactorTooLow,
    
    #[msg("Withdrawal amount exceeds available balance")]
    WithdrawalExceedsBalance,
    
    #[msg("Borrow amount exceeds allowed limit")]
    BorrowExceedsLimit,
    
    #[msg("Position cannot be liquidated - health factor above threshold")]
    CannotLiquidate,
    
    #[msg("Leverage exceeds maximum allowed")]
    LeverageExceedsMaximum,
    
    #[msg("Invalid serum market")]
    InvalidSerumMarket,
    
    #[msg("Invalid oracle price data")]
    InvalidOracleData,
    
    #[msg("Pool already initialized")]
    PoolAlreadyInitialized,
    
    #[msg("Unauthorized access")]
    Unauthorized,
    
    #[msg("Invalid parameter")]
    InvalidParameter,
    
    #[msg("Collateral position not found")]
    CollateralNotFound,
    
    #[msg("Borrow position not found")]
    BorrowNotFound,
    
    #[msg("Leveraged position not found")]
    PositionNotFound,
    
    #[msg("Position is already closed")]
    PositionAlreadyClosed,
    
    #[msg("Position is not eligible for liquidation")]
    PositionNotLiquidatable,
    
    #[msg("Maximum number of leveraged positions reached")]
    MaxPositionsReached,
    
    #[msg("Serum DEX error")]
    SerumDexError,
    
    #[msg("Order placement failed")]
    OrderPlacementFailed,
    
    #[msg("Price slippage exceeded limit")]
    PriceSlippageExceeded,
    
    #[msg("Max leverage for market exceeded")]
    MaxLeverageExceeded,

    // New error types for edge cases
    #[msg("Operation temporarily paused")]
    OperationPaused,
    
    #[msg("Pool utilization rate too high for this operation")]
    UtilizationTooHigh,
    
    #[msg("Minimum lending duration not met")]
    MinLendingDurationNotMet,
    
    #[msg("Lending not enabled for this pool")]
    LendingNotEnabled,
    
    #[msg("Maximum lending capacity reached")]
    MaxLendingCapacityReached,
    
    #[msg("Invalid oracle configuration")]
    InvalidOracleConfig,
    
    #[msg("Stale oracle price data")]
    StaleOracleData,
    
    #[msg("Lending position already exists")]
    LendingPositionAlreadyExists,
    
    #[msg("Insufficient available balance for lending")]
    InsufficientAvailableForLending,
    
    #[msg("Account not authorized for this operation")]
    AccountNotAuthorized,
    
    #[msg("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[msg("Transaction size exceeds limits")]
    TransactionSizeExceeded,
    
    #[msg("Invalid transaction sequence")]
    InvalidTransactionSequence,
    
    #[msg("Insufficient reserves to cover operation")]
    InsufficientReserves,
    
    #[msg("Protocol-wide debt ceiling reached")]
    DebtCeilingReached,
    
    #[msg("Position recently modified, retry after cooldown")]
    PositionModificationCooldown,
}