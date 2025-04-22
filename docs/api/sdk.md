# Oxygen Protocol SDK Documentation

This document provides comprehensive documentation for the Oxygen Protocol SDK, which allows developers to integrate with and build applications on top of the protocol.

## Installation

```bash
npm install @oxygen-protocol/sdk
```

## Core Components

The SDK consists of several key components:

1. **OxygenClient**: Main client for interacting with the protocol
2. **Wallet Adapters**: For connecting with Solana wallets (Phantom, Solflare, etc.)
3. **Position Management**: Tools for managing user positions
4. **Trading Utilities**: Helpers for leveraged trading operations

## OxygenClient

The `OxygenClient` is the main entry point for interacting with the protocol.

### Initialization

```typescript
import { Connection, clusterApiUrl } from '@solana/web3.js';
import { getWalletAdapter } from '@oxygen-protocol/sdk';
import { OxygenClient } from '@oxygen-protocol/sdk';

// Connect to Solana
const connection = new Connection(clusterApiUrl('mainnet-beta'), 'confirmed');

// Get wallet adapter
const walletAdapter = getWalletAdapter();

// Connect wallet (this will prompt user for approval)
await walletAdapter.connect();

// Create client
const oxygenClient = new OxygenClient(connection, walletAdapter);
```

### Core Methods

#### Deposit

Deposits tokens into the protocol:

```typescript
async deposit(
  poolAddress: PublicKey,
  tokenMint: PublicKey,
  amount: BN,
  useAsCollateral: boolean,
  enableLending: boolean
): Promise<string> // Returns transaction signature
```

#### Withdraw

Withdraws tokens from the protocol:

```typescript
async withdraw(
  poolAddress: PublicKey,
  tokenMint: PublicKey,
  amount: BN,
  isLendingWithdrawal: boolean
): Promise<string>
```

#### Borrow

Borrows tokens using collateral:

```typescript
async borrow(
  poolAddress: PublicKey,
  tokenMint: PublicKey,
  amount: BN,
  maintainCollateralLending: boolean
): Promise<string>
```

#### Repay

Repays borrowed tokens:

```typescript
async repay(
  poolAddress: PublicKey,
  tokenMint: PublicKey,
  amount: BN
): Promise<string>
```

#### Get User Position

Retrieves user position information:

```typescript
async getUserPosition(): Promise<any>
```

#### Leveraged Trading

Opens a leveraged trading position:

```typescript
async openLeveragedPosition(
  marketAddress: PublicKey,
  baseTokenMint: PublicKey,
  quoteTokenMint: PublicKey,
  leverage: number,
  size: BN,
  price: BN,
  side: 'buy' | 'sell'
): Promise<string>
```

Closes a leveraged trading position:

```typescript
async closeLeveragedPosition(
  positionId: string
): Promise<string>
```

#### Yield Management

Claim accumulated yield:

```typescript
async claimYield(
  poolAddress: PublicKey,
  tokenMint: PublicKey
): Promise<string>
```

## Wallet Adapters

The SDK provides adapters for popular Solana wallets:

### Available Adapters

- `PhantomWalletAdapter`: For Phantom wallet
- `SolflareWalletAdapter`: For Solflare wallet

### Using Wallet Adapters

```typescript
// Auto-detect and use the appropriate wallet
const walletAdapter = getWalletAdapter();

// Or specify a specific wallet adapter
const phantomAdapter = new PhantomWalletAdapter();
const solflareAdapter = new SolflareWalletAdapter();
```

### Wallet Adapter Interface

All wallet adapters implement the `WalletAdapter` interface:

```typescript
interface WalletAdapter {
  publicKey: PublicKey | null;
  connected: boolean;
  walletName: string;
  
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  signTransaction(transaction: Transaction): Promise<Transaction>;
  signAllTransactions(transactions: Transaction[]): Promise<Transaction[]>;
  signMessage(message: Uint8Array): Promise<Uint8Array>;
}
```

## Advanced Features

### Health Factor Management

Monitor and manage position health:

```typescript
// Get health warnings for positions at risk
const warnings = await oxygenClient.getHealthWarnings();

// Get improvement options for positions at risk
const improvements = await oxygenClient.getHealthImprovementOptions();
```

### Portfolio Analytics

Analyze your portfolio performance:

```typescript
const analytics = await oxygenClient.getPortfolioAnalytics();
console.log('Portfolio APY:', analytics.totalApy);
console.log('Asset allocation:', analytics.allocation);
```

### Protocol Information

Get information about protocol pools:

```typescript
const pools = await oxygenClient.getPools();
const poolInfo = await oxygenClient.getPoolInfo(poolAddress);
```

## Error Handling

The SDK uses a consistent error handling approach:

```typescript
try {
  await oxygenClient.deposit(/* ... */);
} catch (error) {
  if (error.code === 'InsufficientBalance') {
    console.error('Insufficient balance for deposit');
  } else if (error.code === 'PoolNotFound') {
    console.error('Pool not found');
  } else {
    console.error('Unexpected error:', error);
  }
}
```

## Examples

See the [examples repository](https://github.com/oxygen-protocol/examples) for complete implementation examples.

## TypeScript Support

The SDK is fully typed, providing excellent IDE support and compile-time type checking.

## Browser Support

The SDK can be used in both Node.js and browser environments.

## Further Resources

- [GitHub Repository](https://github.com/oxygen-protocol/oxygen)
- [Tutorials](../tutorials/README.md)
- [Architecture Overview](../architecture/overview.md)