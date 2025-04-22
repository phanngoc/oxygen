# Oxygen Protocol: Basic Usage Tutorial

This tutorial will guide you through the fundamental operations of the Oxygen Protocol: depositing assets, borrowing against your collateral, and withdrawing funds—all while maintaining complete control over your assets.

## Prerequisites

- A [Solana wallet](../guides/wallet-setup.md) (Phantom or Solflare recommended)
- SOL for transaction fees
- Tokens to deposit (e.g., USDC, SOL, etc.)

## 1. Connecting Your Wallet

The first step is connecting your wallet to the Oxygen Protocol:

```javascript
// Using the Oxygen SDK
import { Connection, clusterApiUrl } from '@solana/web3.js';
import { getWalletAdapter } from '@oxygen-protocol/sdk';
import { OxygenClient } from '@oxygen-protocol/sdk';

// Connect to Solana
const connection = new Connection(clusterApiUrl('mainnet-beta'), 'confirmed');

// Get the appropriate wallet adapter
const walletAdapter = getWalletAdapter();

// Connect your wallet (this will trigger a wallet popup)
await walletAdapter.connect();

// Create the Oxygen client with the connected wallet
const oxygenClient = new OxygenClient(connection, walletAdapter);
```

## 2. Depositing Assets

To deposit assets into the protocol:

```javascript
// Define the pool and token you want to interact with
const poolAddress = new PublicKey('pool_address_here');
const tokenMint = new PublicKey('token_mint_address_here');

// Deposit 10 tokens (adjust decimals based on the token)
const depositAmount = new BN(10 * 10**9); // For a token with 9 decimals

// Deposit with options to use as collateral and enable lending
const txSignature = await oxygenClient.deposit(
  poolAddress,
  tokenMint,
  depositAmount,
  true,  // use as collateral
  true   // enable lending
);

console.log(`Deposit successful! Transaction: ${txSignature}`);
```

## 3. Borrowing Against Your Collateral

Once you've deposited assets as collateral, you can borrow other assets:

```javascript
// Define the pool and token you want to borrow
const borrowPoolAddress = new PublicKey('borrow_pool_address_here');
const borrowTokenMint = new PublicKey('borrow_token_mint_here');

// Borrow 5 tokens
const borrowAmount = new BN(5 * 10**9); // For a token with 9 decimals

// Borrow with option to maintain lending position
const txSignature = await oxygenClient.borrow(
  borrowPoolAddress,
  borrowTokenMint,
  borrowAmount,
  true  // maintain collateral lending
);

console.log(`Borrow successful! Transaction: ${txSignature}`);
```

## 4. Checking Your Position

You can check your current position within the protocol:

```javascript
// Fetch your position details
const position = await oxygenClient.getUserPosition();

console.log('Current position:', position);
// This shows your deposits, loans, and health factor
```

## 5. Repaying Your Loan

To repay borrowed assets:

```javascript
// Define the pool and token for repayment
const repayPoolAddress = new PublicKey('pool_address_here');
const repayTokenMint = new PublicKey('token_mint_address_here');

// Repay 2 tokens
const repayAmount = new BN(2 * 10**9); // For a token with 9 decimals

// Repay your loan
const txSignature = await oxygenClient.repay(
  repayPoolAddress,
  repayTokenMint,
  repayAmount
);

console.log(`Repayment successful! Transaction: ${txSignature}`);
```

## 6. Withdrawing Your Assets

When you're ready to withdraw your assets:

```javascript
// Define the pool and token for withdrawal
const withdrawPoolAddress = new PublicKey('pool_address_here');
const withdrawTokenMint = new PublicKey('token_mint_address_here');

// Withdraw 3 tokens
const withdrawAmount = new BN(3 * 10**9); // For a token with 9 decimals

// Withdraw your assets
const txSignature = await oxygenClient.withdraw(
  withdrawPoolAddress,
  withdrawTokenMint,
  withdrawAmount,
  false  // not a lending withdrawal
);

console.log(`Withdrawal successful! Transaction: ${txSignature}`);
```

## 7. Disconnecting Your Wallet

When you're done using the protocol:

```javascript
// Disconnect your wallet
await walletAdapter.disconnect();
console.log('Disconnected from wallet');
```

## Understanding Health Factor

The health factor represents the safety of your position against liquidation:

- Health factor > 1: Your position is safe
- Health factor < 1: Your position can be liquidated

You can improve your health factor by:
1. Depositing more collateral
2. Repaying part of your loan
3. Reducing your borrowed amount

## Next Steps

- [Advanced Tutorial](./advanced-usage.md): Learn about yield generation and leveraged trading
- [API Reference](../api/sdk.md): Explore the full SDK capabilities
- [Architecture Overview](../architecture/overview.md): Understand how the protocol works