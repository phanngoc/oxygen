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
}