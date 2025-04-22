use anchor_lang::prelude::*;

#[account]
pub struct Pool {
    pub asset_mint: Pubkey,              // Token mint address
    pub asset_reserve: Pubkey,           // Pool's token account
    pub total_deposits: u64,             // Total deposits in the pool
    pub total_borrows: u64,              // Total borrows from the pool
    pub cumulative_borrow_rate: u128,    // Accumulated borrow rate
    pub last_updated: i64,               // Last update timestamp
    pub optimal_utilization: u64,        // Target utilization rate
    pub loan_to_value: u64,              // Max LTV ratio for this asset
    pub liquidation_threshold: u64,      // Liquidation threshold
    pub liquidation_bonus: u64,          // Bonus for liquidators
    pub borrow_fee: u64,                 // Fee for borrowing
    pub flash_loan_fee: u64,             // Fee for flash loans
    pub host_fee_percentage: u8,         // Host fee percentage
    pub protocol_fee_percentage: u8,     // Protocol fee percentage
    pub bump: u8,                        // PDA bump
}

impl Pool {
    pub fn space() -> usize {
        8 + // Anchor account discriminator
        32 + // asset_mint
        32 + // asset_reserve
        8 + // total_deposits
        8 + // total_borrows
        16 + // cumulative_borrow_rate
        8 + // last_updated
        8 + // optimal_utilization
        8 + // loan_to_value
        8 + // liquidation_threshold
        8 + // liquidation_bonus
        8 + // borrow_fee
        8 + // flash_loan_fee
        1 + // host_fee_percentage
        1 + // protocol_fee_percentage
        1 // bump
    }

    pub fn update_rates(&mut self, current_timestamp: i64) -> Result<()> {
        // Update interest rates based on pool utilization
        if self.total_deposits == 0 {
            return Ok(());
        }

        let utilization_rate = if self.total_deposits > 0 {
            (self.total_borrows as u128).checked_mul(10000).unwrap_or(0) / (self.total_deposits as u128)
        } else {
            0
        };

        // Simple interest rate model based on utilization
        // More sophisticated models can be implemented later
        let borrow_rate = if utilization_rate < (self.optimal_utilization as u128) {
            // Below optimal: lower rate
            utilization_rate.checked_mul(10).unwrap_or(0) / 100
        } else {
            // Above optimal: increase rate more aggressively
            let base_rate = (self.optimal_utilization as u128).checked_mul(10).unwrap_or(0) / 100;
            let excess_utilization = utilization_rate.checked_sub(self.optimal_utilization as u128).unwrap_or(0);
            let excess_rate = excess_utilization.checked_mul(20).unwrap_or(0) / 100;
            base_rate.checked_add(excess_rate).unwrap_or(0)
        };

        // Time elapsed since last update (in seconds)
        let time_elapsed = (current_timestamp - self.last_updated) as u128;
        
        // Update cumulative borrow rate
        // Formula: previous_rate + (borrow_rate * time_elapsed / SECONDS_PER_YEAR)
        const SECONDS_PER_YEAR: u128 = 31536000; // 365 * 24 * 60 * 60
        
        let rate_increase = borrow_rate
            .checked_mul(time_elapsed).unwrap_or(0)
            .checked_div(SECONDS_PER_YEAR).unwrap_or(0);
            
        self.cumulative_borrow_rate = self.cumulative_borrow_rate
            .checked_add(rate_increase).unwrap_or(self.cumulative_borrow_rate);
            
        // Update timestamp
        self.last_updated = current_timestamp;
        
        Ok(())
    }

    pub fn get_utilization_rate(&self) -> u64 {
        if self.total_deposits == 0 {
            return 0;
        }
        
        ((self.total_borrows as u128).checked_mul(10000).unwrap_or(0) / (self.total_deposits as u128)) as u64
    }
}