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
}