use anchor_lang::prelude::*;

#[account]
pub struct MarketInfo {
    pub serum_market: Pubkey,            // Serum market address
    pub asset_mint: Pubkey,              // Base token mint
    pub quote_mint: Pubkey,              // Quote token mint
    pub oracle: Pubkey,                  // Price oracle address
    pub optimal_leverage: u64,           // Recommended max leverage
    pub max_leverage: u64,               // Maximum allowed leverage
    pub liquidation_fee: u64,            // Fee during liquidations
    pub maintenance_margin_ratio: u64,   // Min required margin
    pub bump: u8,                        // PDA bump
}

impl MarketInfo {
    pub fn space() -> usize {
        8 + // Anchor account discriminator
        32 + // serum_market
        32 + // asset_mint
        32 + // quote_mint
        32 + // oracle
        8 + // optimal_leverage
        8 + // max_leverage
        8 + // liquidation_fee
        8 + // maintenance_margin_ratio
        1   // bump
    }
    
    pub fn is_leverage_valid(&self, requested_leverage: u64) -> bool {
        requested_leverage <= self.max_leverage
    }
    
    pub fn calculate_margin_requirement(&self, position_size: u64, price: u64) -> Result<u64> {
        // Calculate position value
        let position_value = (position_size as u128)
            .checked_mul(price as u128)
            .ok_or(ErrorCode::MathOverflow)?;
        
        // Calculate required margin using maintenance margin ratio
        let required_margin = position_value
            .checked_mul(self.maintenance_margin_ratio as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000) // Assuming margin ratio is in basis points (e.g., 500 = 5%)
            .ok_or(ErrorCode::MathOverflow)?;
            
        Ok(required_margin as u64)
    }
}