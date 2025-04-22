import { Connection, PublicKey, clusterApiUrl } from '@solana/web3.js';
import { getWalletAdapter, PhantomWalletAdapter, SolflareWalletAdapter } from './index';
import { OxygenClient } from './client';
import { BN } from 'bn.js';

/**
 * Example of using the Oxygen protocol in a completely non-custodial way
 * This demonstrates how users maintain full control of their funds
 */
async function nonCustodialExample() {
  try {
    console.log('Starting non-custodial Oxygen protocol example...');
    
    // Connect to Solana
    const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');
    
    // Get the appropriate wallet adapter based on what's available in the browser
    // This handles both Phantom and Solflare wallets
    const walletAdapter = getWalletAdapter();
    
    console.log(`Found wallet: ${walletAdapter.walletName}`);
    
    // Connect to the wallet - this will prompt the user to approve the connection
    // Non-custodial: The wallet remains in the user's control at all times
    await walletAdapter.connect();
    
    if (!walletAdapter.publicKey) {
      throw new Error('Failed to connect wallet');
    }
    
    console.log(`Connected to wallet: ${walletAdapter.publicKey.toString()}`);
    
    // Create the Oxygen client with the connected wallet
    const oxygenClient = new OxygenClient(connection, walletAdapter);
    
    // Example pool and token addresses (these would be actual addresses in production)
    const poolAddress = new PublicKey('11111111111111111111111111111111');
    const tokenMint = new PublicKey('22222222222222222222222222222222');
    
    // Example of depositing tokens
    // NON-CUSTODIAL: User must sign this transaction with their wallet
    console.log('Depositing tokens...');
    const depositAmount = new BN(1000000000); // 1 token with 9 decimals
    const depositTxSignature = await oxygenClient.deposit(
      poolAddress,
      tokenMint,
      depositAmount,
      true, // use as collateral
      true  // enable lending
    );
    console.log(`Deposit successful! Transaction: ${depositTxSignature}`);
    
    // Example of borrowing tokens
    // NON-CUSTODIAL: User must sign this transaction with their wallet
    console.log('Borrowing tokens...');
    const borrowAmount = new BN(500000000); // 0.5 token with 9 decimals
    const borrowTxSignature = await oxygenClient.borrow(
      poolAddress,
      tokenMint,
      borrowAmount,
      true // maintain collateral lending
    );
    console.log(`Borrow successful! Transaction: ${borrowTxSignature}`);
    
    // Get user position information
    console.log('Fetching user position...');
    const userPosition = await oxygenClient.getUserPosition();
    console.log('User position:', userPosition);
    
    // Example of repaying borrowed tokens
    // NON-CUSTODIAL: User must sign this transaction with their wallet
    console.log('Repaying borrowed tokens...');
    const repayAmount = new BN(100000000); // 0.1 token with 9 decimals
    const repayTxSignature = await oxygenClient.repay(
      poolAddress,
      tokenMint,
      repayAmount
    );
    console.log(`Repay successful! Transaction: ${repayTxSignature}`);
    
    // Example of withdrawing tokens
    // NON-CUSTODIAL: User must sign this transaction with their wallet
    console.log('Withdrawing tokens...');
    const withdrawAmount = new BN(200000000); // 0.2 token with 9 decimals
    const withdrawTxSignature = await oxygenClient.withdraw(
      poolAddress,
      tokenMint,
      withdrawAmount,
      false // not a lending withdrawal
    );
    console.log(`Withdraw successful! Transaction: ${withdrawTxSignature}`);
    
    // Disconnect from wallet
    await walletAdapter.disconnect();
    console.log('Disconnected from wallet');
    
  } catch (error) {
    console.error('Error:', error);
  }
}

// For browser environments, run when the page loads
if (typeof window !== 'undefined') {
  window.addEventListener('load', () => {
    // Add a button to the page that triggers the example
    const button = document.createElement('button');
    button.textContent = 'Run Non-Custodial Example';
    button.addEventListener('click', nonCustodialExample);
    document.body.appendChild(button);
  });
}

// For Node.js environments, export the example function
export { nonCustodialExample };