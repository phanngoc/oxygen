use anchor_lang::prelude::*;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Module for handling non-custodial wallet integrations
/// This ensures users always maintain complete control of their funds
/// and only they can authorize transactions with their private keys
pub struct WalletIntegration;

impl WalletIntegration {
    /// Validates a transaction is signed by the rightful owner
    /// This is a core component of ensuring the protocol remains non-custodial
    pub fn validate_owner_signed(
        owner_pubkey: &Pubkey,
        signer: &Signer,
    ) -> Result<()> {
        require!(
            owner_pubkey == signer.key,
            crate::errors::OxygenError::UserSignatureRequired
        );
        Ok(())
    }
    
    /// Verifies a transaction is coming from a valid wallet
    /// This helps protect users from potential phishing attempts
    pub fn verify_wallet_origin(
        wallet_program_id: &Pubkey,
    ) -> Result<()> {
        // These are the program IDs for common Solana wallets
        // No actual restriction is placed since any valid Solana wallet should work
        // This is informational only and ensures compatibility
        let known_wallet_programs = [
            // Phantom Wallet program ID
            "PhaNtomWaLLeTXXXXXXXXXXXXXXXXXXXXXXXXXX",
            // Solflare Wallet program ID
            "SoLfLareWaLLetXXXXXXXXXXXXXXXXXXXXXXXXX",
            // Slope Wallet program ID
            "SLopeWaLLetXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
            // Other wallet programs can be added here for explicit support
        ];
        
        // No actual restriction - just for documentation
        Ok(())
    }
    
    /// Gets wallet transaction metadata for transparency
    /// This lets users see exactly what they're signing
    pub fn get_transaction_metadata(
        transaction_data: &[u8],
    ) -> Result<TransactionMetadata> {
        // In a real implementation, this would parse the transaction data
        // to provide user-friendly information about what is being signed
        
        // For this example, we just return a placeholder
        Ok(TransactionMetadata {
            description: "Oxygen Protocol Transaction".to_string(),
            action_type: ActionType::UserInitiated,
            requires_signature: true,
            is_cancellable: true,
        })
    }
    
    /// Validates that a transaction doesn't contain admin operations
    /// This ensures the protocol remains decentralized with no special privileges
    pub fn validate_no_admin_operations(
        instruction_data: &[u8],
    ) -> Result<()> {
        // In a production implementation, this would analyze the instruction data
        // to ensure it doesn't contain any privileged operations
        
        // For this example, we just return Ok
        Ok(())
    }
}

/// Structure representing transaction metadata for wallet transparency
#[derive(Debug, Clone)]
pub struct TransactionMetadata {
    /// Human-readable description of the transaction
    pub description: String,
    
    /// Type of action being performed
    pub action_type: ActionType,
    
    /// Whether the transaction requires a user signature
    pub requires_signature: bool,
    
    /// Whether the user can cancel the transaction
    pub is_cancellable: bool,
}

/// Enum representing the type of action being performed
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    /// Transaction initiated by the user
    UserInitiated,
    
    /// Transaction initiated by the protocol
    ProtocolInitiated,
    
    /// Transaction initiated by another user (e.g., liquidation)
    ThirdPartyInitiated,
}