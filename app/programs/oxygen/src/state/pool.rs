use anchor_lang::prelude::*;

#[account]
pub struct Pool {
    pub asset_mint: Pubkey,              // Token mint address
    pub asset_reserve: Pubkey,           // Pool's token account
    pub total_deposits: u64,             // Total deposits in the pool
    pub total_borrows: u64,              // Total borrows from the pool
    pub available_lending_supply: u64,   // Amount available to be lent out
    pub cumulative_borrow_rate: u128,    // Accumulated borrow rate
    pub cumulative_lending_rate: u128,   // Accumulated lending rate
    pub last_updated: i64,               // Last update timestamp
    pub optimal_utilization: u64,        // Target utilization rate
    pub loan_to_value: u64,              // Max LTV ratio for this asset
    pub liquidation_threshold: u64,      // Liquidation threshold
    pub liquidation_bonus: u64,          // Bonus for liquidators
    pub borrow_fee: u64,                 // Fee for borrowing
    pub flash_loan_fee: u64,             // Fee for flash loans
    pub host_fee_percentage: u8,         // Host fee percentage
    pub protocol_fee_percentage: u8,     // Protocol fee percentage
    pub lending_enabled: bool,           // Whether lending is enabled
    pub max_lending_ratio: u64,          // Maximum % of deposits for lending
    pub min_lending_duration: u64,       // Minimum duration for lending
    pub lending_fee: u64,                // Fee for lending (bps)
    pub lending_interest_share: u64,     // % of interest to lenders
    pub total_lent: u64,                 // Total amount being lent
    pub operation_state_flags: u8,       // Flags for pausing operations
    pub price_oracle: Pubkey,            // Oracle account for price feeds
    pub last_oracle_price: u64,          // Last recorded oracle price
    pub last_oracle_update: i64,         // Timestamp of last oracle update
    pub bump: u8,                        // PDA bump

    /// Track individual user deposits in a PDA-based mapping
    pub user_deposits_authority: Pubkey,
    
    /// Non-upgradable flag ensures protocol cannot be changed after deployment
    pub immutable: bool,
    
    /// Flag to indicate this pool was initialized without admin keys
    pub admin_less: bool,
}

impl Pool {
    pub fn space() -> usize {
        8 + // Anchor account discriminator
        32 + // asset_mint
        32 + // asset_reserve
        8 + // total_deposits
        8 + // total_borrows
        8 + // available_lending_supply
        16 + // cumulative_borrow_rate
        16 + // cumulative_lending_rate
        8 + // last_updated
        8 + // optimal_utilization
        8 + // loan_to_value
        8 + // liquidation_threshold
        8 + // liquidation_bonus
        8 + // borrow_fee
        8 + // flash_loan_fee
        1 + // host_fee_percentage
        1 + // protocol_fee_percentage
        1 + // lending_enabled
        8 + // max_lending_ratio
        8 + // min_lending_duration
        8 + // lending_fee
        8 + // lending_interest_share
        8 + // total_lent
         1 + // operation_state_flags
        32 + // price_oracle
        8 + // last_oracle_price
        8 + // last_oracle_update
         1 + // bump
        32 + // user_deposits_authority
        1 + // immutable
        1   // admin_less
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

    pub fn deposit_to_scaled(&self, amount: u64) -> Result<u128> {
        // Convert deposit amount to scaled amount based on the current exchange rate
        if self.total_deposits == 0 {
            // First deposit, 1:1 ratio
            return Ok(amount as u128);
        }
        
        // Scale by cumulative lending rate
        // scaled_amount = amount * 10^12 / cumulative_lending_rate
        let scaled_amount = (amount as u128)
            .checked_mul(1_000_000_000_000) // 10^12 precision
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(self.cumulative_lending_rate)
            .ok_or(ErrorCode::MathOverflow)?;
            
        Ok(scaled_amount)
    }
    
    pub fn update_utilization_rate(&mut self) -> Result<()> {
        // Calculate the current utilization rate of the pool
        if self.total_deposits == 0 {
            // No deposits, zero utilization
            return Ok(());
        }
        
        // Also update available for lending based on lending flags
        // This function should be called after deposit/withdraw/borrow/repay operations
        let lending_utilization = if self.available_lending_supply > 0 {
            (self.total_borrows as u128)
                .checked_mul(10000)
                .unwrap_or(0) / (self.available_lending_supply as u128)
        } else {
            0
        };
        
        // Update lending rate based on lending utilization
        // This determines the yield distributed to lenders
        if self.last_updated > 0 {
            let utilization_factor = std::cmp::min(lending_utilization as u64, 10000);
            let base_lending_rate = (utilization_factor as u128)
                .checked_mul(8) // 80% of borrow rate goes to lenders
                .unwrap_or(0)
                .checked_div(10)
                .unwrap_or(0);
                
            // Update cumulative lending rate
            const SECONDS_PER_YEAR: u128 = 31536000; // 365 * 24 * 60 * 60
            let time_elapsed = (Clock::get().unwrap().unix_timestamp - self.last_updated) as u128;
            
            let rate_increase = base_lending_rate
                .checked_mul(time_elapsed).unwrap_or(0)
                .checked_div(SECONDS_PER_YEAR).unwrap_or(0);
                
            self.cumulative_lending_rate = self.cumulative_lending_rate
                .checked_add(rate_increase).unwrap_or(self.cumulative_lending_rate);
        }
        
        Ok(())
    }

    // Get the current borrow interest rate for the pool
    pub fn get_borrow_rate(&self) -> Result<u64> {
        let utilization_rate = self.get_utilization_rate();
        
        // Using the same interest model as in update_rates
        let borrow_rate = if utilization_rate < self.optimal_utilization {
            // Below optimal: lower rate
            (utilization_rate as u128)
                .checked_mul(10)
                .unwrap_or(0)
                .checked_div(100)
                .unwrap_or(0) as u64
        } else {
            // Above optimal: increase rate more aggressively
            let base_rate = (self.optimal_utilization as u128)
                .checked_mul(10)
                .unwrap_or(0)
                .checked_div(100)
                .unwrap_or(0) as u64;
                
            let excess_utilization = utilization_rate
                .checked_sub(self.optimal_utilization)
                .unwrap_or(0);
                
            let excess_rate = (excess_utilization as u128)
                .checked_mul(20)
                .unwrap_or(0)
                .checked_div(100)
                .unwrap_or(0) as u64;
                
            base_rate
                .checked_add(excess_rate)
                .ok_or(ErrorCode::MathOverflow)?
        };
        
        Ok(borrow_rate)
    }
    
    // Get the current lending interest rate for the pool
    pub fn get_lending_rate(&self) -> Result<u64> {
        // Lending rate is a percentage of the borrow rate
        // determined by the lending_interest_share parameter
        let borrow_rate = self.get_borrow_rate()?;
        
        let lending_rate = (borrow_rate as u128)
            .checked_mul(self.lending_interest_share as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(10000)
            .ok_or(ErrorCode::MathOverflow)? as u64;
            
        Ok(lending_rate)
    }

    /// Verify a transaction is authorized by the rightful owner
    pub fn verify_owner_signed(&self, signer: &Signer) -> Result<()> {
        require!(
            self.user_deposits_authority == signer.key(),
            OxygenError::Unauthorized
        );
        Ok(())
    }
    
    /// Ensure pools can never be upgraded or changed by admins
    pub fn verify_immutable(&self) -> Result<()> {
        require!(self.immutable, OxygenError::PoolIsUpgradable);
        Ok(())
    }
}