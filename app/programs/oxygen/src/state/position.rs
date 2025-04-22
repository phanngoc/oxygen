use anchor_lang::prelude::*;
use std::collections::HashMap;

/// User position in the protocol
#[account]
#[derive(Default)]
pub struct UserPosition {
    pub owner: Pubkey,                              // User wallet
    pub collaterals: Vec<CollateralPosition>,       // User collaterals
    pub borrows: Vec<BorrowPosition>,               // User borrows
    pub leveraged_positions: Vec<LeveragedPosition>, // User's leveraged trading positions
    pub health_factor: u64,                         // Current health factor
    pub last_updated: i64,                          // Last update timestamp
    pub bump: u8,                                   // PDA bump
}

/// Status of a leveraged position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq)]
pub enum PositionStatus {
    Open,       // Position is open
    Closed,     // Position was closed by the user
    Liquidated, // Position was liquidated due to insufficient margin
}

impl Default for PositionStatus {
    fn default() -> Self {
        PositionStatus::Open
    }
}

/// Leveraged trading position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct LeveragedPosition {
    pub id: u64,                     // Unique position identifier
    pub market: Pubkey,              // Market address (Serum DEX market)
    pub side: crate::instructions::OrderSide, // Buy or sell side
    pub size: u64,                   // Position size
    pub entry_price: u64,            // Entry price
    pub leverage: u64,               // Leverage used (in basis points, 10000 = 1x)
    pub margin_used: u64,            // Margin used for this position
    pub position_value: u64,         // Total value of the position
    pub liquidation_price: u64,      // Price at which position will be liquidated
    pub timestamp: i64,              // Time when position was opened
    pub status: PositionStatus,      // Current status of the position
    pub client_id: u64,              // Client order ID for identification
}

/// Collateral position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct CollateralPosition {
    pub pool: Pubkey,                // Pool address
    pub amount_deposited: u64,       // Deposited amount
    pub amount_scaled: u128,         // Scaled amount (for yield)
    pub is_collateral: bool,         // Used as collateral
}

/// Borrow position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct BorrowPosition {
    pub pool: Pubkey,                // Pool address
    pub amount_borrowed: u64,        // Borrowed amount
    pub amount_scaled: u128,         // Scaled amount (for interest)
    pub interest_rate: u64,          // Interest rate at time of borrow
}

impl UserPosition {
    pub const MAX_COLLATERALS: usize = 10;
    pub const MAX_BORROWS: usize = 10;
    
    pub fn space() -> usize {
        8 + // Anchor account discriminator
        32 + // owner
        4 + (Self::MAX_COLLATERALS * std::mem::size_of::<CollateralPosition>()) + // collaterals vector
        4 + (Self::MAX_BORROWS * std::mem::size_of::<BorrowPosition>()) + // borrows vector
        8 + // health_factor
        8 + // last_updated
        1  // bump
    }
    
    pub fn add_collateral(&mut self, pool: Pubkey, amount: u64, scaled_amount: u128) -> Result<()> {
        // Check if we already have this collateral
        for collateral in &mut self.collaterals {
            if collateral.pool == pool {
                // Update existing collateral position
                collateral.amount_deposited = collateral.amount_deposited.checked_add(amount)
                    .ok_or(ErrorCode::MathOverflow)?;
                collateral.amount_scaled = collateral.amount_scaled.checked_add(scaled_amount)
                    .ok_or(ErrorCode::MathOverflow)?;
                collateral.is_collateral = true;
                return Ok(());
            }
        }
        
        // Add new collateral if not found and we have space
        if self.collaterals.len() < Self::MAX_COLLATERALS {
            self.collaterals.push(CollateralPosition {
                pool,
                amount_deposited: amount,
                amount_scaled: scaled_amount,
                is_collateral: true,
            });
            return Ok(());
        }
        
        // No space for new collateral
        Err(ErrorCode::AccountDidNotSerialize.into())
    }
    
    pub fn add_borrow(&mut self, pool: Pubkey, amount: u64, scaled_amount: u128, interest_rate: u64) -> Result<()> {
        // Check if we already have this borrow
        for borrow in &mut self.borrows {
            if borrow.pool == pool {
                // Update existing borrow position
                borrow.amount_borrowed = borrow.amount_borrowed.checked_add(amount)
                    .ok_or(ErrorCode::MathOverflow)?;
                borrow.amount_scaled = borrow.amount_scaled.checked_add(scaled_amount)
                    .ok_or(ErrorCode::MathOverflow)?;
                return Ok(());
            }
        }
        
        // Add new borrow if not found and we have space
        if self.borrows.len() < Self::MAX_BORROWS {
            self.borrows.push(BorrowPosition {
                pool,
                amount_borrowed: amount,
                amount_scaled: scaled_amount,
                interest_rate,
            });
            return Ok(());
        }
        
        // No space for new borrow
        Err(ErrorCode::AccountDidNotSerialize.into())
    }
    
    // Calculate health factor based on collateral value and borrowed amounts
    // Health factor = (collateral value * liquidation threshold) / borrowed value
    pub fn calculate_health_factor(&mut self, pool_data: &HashMap<Pubkey, (u64, u64)>) -> Result<u64> {
        let mut total_collateral_value = 0u128;
        let mut total_borrowed_value = 0u128;
        
        // Calculate collateral value
        for collateral in &self.collaterals {
            if !collateral.is_collateral {
                continue;
            }
            
            if let Some((price, liquidation_threshold)) = pool_data.get(&collateral.pool) {
                let value = (collateral.amount_deposited as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                
                let weighted_value = value
                    .checked_mul(*liquidation_threshold as u128)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(10000)
                    .ok_or(ErrorCode::MathOverflow)?;
                
                total_collateral_value = total_collateral_value
                    .checked_add(weighted_value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        // Calculate borrowed value
        for borrow in &self.borrows {
            if let Some((price, _)) = pool_data.get(&borrow.pool) {
                let value = (borrow.amount_borrowed as u128)
                    .checked_mul(*price as u128)
                    .ok_or(ErrorCode::MathOverflow)?;
                
                total_borrowed_value = total_borrowed_value
                    .checked_add(value)
                    .ok_or(ErrorCode::MathOverflow)?;
            }
        }
        
        // Include leveraged positions in the risk calculation
        for position in &self.leveraged_positions {
            // Only consider open positions
            if position.status != PositionStatus::Open {
                continue;
            }
            
            // A leveraged position adds risk proportional to the leveraged value minus margin
            let leveraged_risk = (position.position_value as u128)
                .checked_sub(position.margin_used as u128)
                .ok_or(ErrorCode::MathOverflow)?;
                
            total_borrowed_value = total_borrowed_value
                .checked_add(leveraged_risk)
                .ok_or(ErrorCode::MathOverflow)?;
        }
        
        // Calculate health factor
        if total_borrowed_value == 0 {
            self.health_factor = u64::MAX; // No borrows, so perfectly healthy
            return Ok(self.health_factor);
        }
        
        // Health factor = (collateral value * liquidation threshold) / borrowed value
        // We multiply by 10000 to preserve precision
        self.health_factor = (total_collateral_value
            .checked_mul(10000)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(total_borrowed_value)
            .ok_or(ErrorCode::MathOverflow)?) as u64;
            
        Ok(self.health_factor)
    }
    
    pub fn is_healthy(&self, minimum_health_factor: u64) -> bool {
        self.health_factor >= minimum_health_factor
    }
}