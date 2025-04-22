# Oxygen Protocol Architecture Overview

This document provides a comprehensive overview of the Oxygen Protocol's architecture, explaining how the system components interact to create a secure, non-custodial DeFi lending and trading platform on Solana.

## Core Principles

The Oxygen Protocol architecture is built on these core principles:

1. **Non-custodial**: Users maintain full control of their funds, only they can sign transactions
2. **Decentralized**: No admin controls or upgradeability hooks that could compromise security
3. **Efficient**: Optimized for Solana's high-performance blockchain environment
4. **Composable**: Designed to integrate with other DeFi protocols in the Solana ecosystem
5. **Secure**: Multiple safeguards against exploits and economic attacks

## System Architecture

![System Architecture](../diagrams/system_architecture.png)

### Key Components

The Oxygen Protocol consists of these major components:

#### 1. Core Protocol (Solana Programs)

- **Pool Module**: Manages lending pools for different assets
- **Position Module**: Tracks user deposits, collateral, and borrows
- **Trading Module**: Handles leveraged trading via integration with Serum DEX
- **Interest Module**: Calculates and accumulates interest for lenders and borrowers
- **Liquidation Module**: Monitors positions and executes liquidations when necessary
- **Yield Module**: Distributes yield to lenders
- **Wallet Integration Module**: Ensures non-custodial operation with wallet security

#### 2. SDK Layer

- **Client Libraries**: JavaScript/TypeScript libraries for interacting with the protocol
- **Wallet Adapters**: Integrations with Phantom, Solflare, and other Solana wallets
- **API Wrappers**: Simplified interfaces for complex protocol operations

#### 3. User Interface

- **Web Application**: Front-end interface for users to interact with the protocol
- **Position Dashboard**: Visualizes user positions, health factors, and yields
- **Trading Interface**: Advanced interface for leveraged trading operations

## Data Flow

User operations follow these general paths:

1. **Deposit Flow**:
   - User signs deposit transaction with their wallet
   - Funds transfer from user's wallet to protocol PDA
   - User position is updated with new deposit
   - If lending is enabled, asset becomes available in lending pool

2. **Borrow Flow**:
   - User's collateral position is validated
   - User signs borrow transaction with their wallet
   - Borrowed funds transfer to user's wallet
   - Interest begins accruing on borrowed amount

3. **Repay Flow**:
   - User signs repayment transaction with their wallet
   - Funds transfer from user's wallet to protocol
   - User's debt position is reduced
   - Interest stops accruing on repaid amount

4. **Withdraw Flow**:
   - User signs withdrawal transaction
   - Collateral usage is validated to prevent insolvency
   - Funds transfer from protocol back to user's wallet

## Security Model

### Non-Custodial Design

The protocol ensures non-custodial operation through:

1. **Mandatory User Signatures**: All transactions involving user funds require the user's signature
2. **No Admin Keys**: Protocol operations have no special admin privileges
3. **Immutable Pools**: Once deployed, pool parameters cannot be changed
4. **Transparent Operations**: All operations occur on-chain with full visibility

### Risk Mitigation

The protocol implements multiple safeguards:

1. **Health Factor Monitoring**: Continuous tracking of collateral-to-debt ratios
2. **Liquidation Thresholds**: Clear thresholds for when positions become eligible for liquidation
3. **Oracle Integration**: Reliable price feeds for accurate collateral valuation
4. **Circuit Breakers**: System safeguards that trigger during extreme market conditions

## Account Structure

The protocol uses several account types:

1. **Pool Accounts**: Store lending pool information and parameters
2. **User Position Accounts**: Track user deposits, borrows, and collateral
3. **Reserve Accounts**: Hold pooled assets for lending and borrowing
4. **Market Accounts**: Interface with Serum DEX for leveraged trading

## Protocol Parameters

Key protocol parameters include:

| Parameter | Description | Typical Value |
|-----------|-------------|---------------|
| Liquidation Threshold | When positions become eligible for liquidation | 80-85% |
| Loan-to-Value Ratio | Maximum borrowing capacity relative to collateral | 60-75% |
| Liquidation Bonus | Extra compensation for liquidators | 5-10% |
| Optimal Utilization | Target utilization rate for interest rate model | 80% |
| Reserve Factor | Portion of interest reserved for protocol | 10-20% |

## Interest Rate Model

The protocol uses a dynamic interest rate model based on pool utilization:

![Interest Rate Model](../diagrams/interest_rate_model.png)

- **Low Utilization**: Lower interest rates to encourage borrowing
- **Optimal Utilization**: Balanced interest rates for lenders and borrowers
- **High Utilization**: Rapidly increasing rates to encourage deposits and discourage borrowing

## Integration Points

The protocol integrates with:

1. **Serum DEX**: For order book-based leveraged trading
2. **Pyth Network**: For oracle price data
3. **Solana SPL Tokens**: For handling various token types

## Next Steps

- [Technical Specifications](./technical_specifications.md): Detailed protocol specifications
- [Smart Contract Details](./smart_contracts.md): In-depth look at the protocol's smart contracts
- [SDK Documentation](../api/sdk.md): How to interact with the protocol programmatically