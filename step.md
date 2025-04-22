tạo các prompt cho cursor AI editor, để phát triển sản phẩm MVP với tính năng : DeFi prime brokerage service built on Solana and powered by Serum's on-chain infrastructure. Built to support 100s of millions of users, it serves as a permissionless, cheap, and scalable protocol that democratizes borrowing, lending, and trading with leverage and allows you to make the most of your capital.

You can earn yield, borrow from peers, trade directly out of your pools, and get trading leverage against a portfolio of assets. It provides a more efficient way to manage capital and is unique from other borrow lending protocols in three ways:
- Multiple uses of the same collateral. App enables you to generate yield on your portfolio through lending out your assets and borrowing other assets at the same time.
- Cross-collateralization. You can utilize all of your portfolio as collateral when you want to borrow other assets, meaning a lower margin call and liquidation risk for your portfolio.
- Market-based pricing. App protocol is order-book based instead of following a pre-set market model that needs to be manually adjusted.
- 100% decentralised, 100% non-custodial, and 100% on-chain. All transactions are purely peer-to-peer with no involvement from a centralized operator. App protocol never has access to your private keys at any point.

# Oxygen Protocol MVP - Project Scaffold

## 1. Project Structure

```
oxygen-protocol/
├── programs/                         # Solana on-chain programs 
│   └── oxygen/                       # Main protocol program
│       ├── src/
│       │   ├── lib.rs                # Program entry point
│       │   ├── state/                # Program state accounts
│       │   │   ├── mod.rs
│       │   │   ├── pool.rs           # Liquidity pool state
│       │   │   ├── position.rs       # User position state
│       │   │   └── market.rs         # Market integration state
│       │   ├── instructions/         # Program instructions
│       │   │   ├── mod.rs
│       │   │   ├── init_pool.rs      # Initialize lending pool
│       │   │   ├── deposit.rs        # Deposit assets
│       │   │   ├── withdraw.rs       # Withdraw assets
│       │   │   ├── borrow.rs         # Borrow assets
│       │   │   ├── repay.rs          # Repay loans
│       │   │   ├── trade.rs          # Trading with leverage
│       │   │   ├── liquidate.rs      # Liquidation logic
│       │   │   └── claim_yield.rs    # Claim generated yield
│       │   ├── modules/              # Core functionality modules
│       │   │   ├── mod.rs
│       │   │   ├── lending.rs        # Lending module
│       │   │   ├── collateral.rs     # Collateral management
│       │   │   ├── trading.rs        # Trading integration
│       │   │   ├── yield.rs          # Yield generation
│       │   │   ├── interest.rs       # Interest rate model
│       │   │   └── liquidation.rs    # Liquidation engine
│       │   └── errors.rs             # Program errors
│       ├── Cargo.toml                # Dependencies
│       └── Xargo.toml                # Rust cross-compilation config
├── app/                              # Frontend client application
│   ├── public/
│   ├── src/
│   │   ├── components/               # React components
│   │   ├── contexts/                 # React contexts
│   │   ├── hooks/                    # Custom hooks
│   │   ├── models/                   # TypeScript interfaces
│   │   ├── services/                 # API services
│   │   └── utils/                    # Utility functions
│   ├── package.json
│   └── tsconfig.json
├── sdk/                              # TypeScript SDK for the protocol
│   ├── src/
│   │   ├── index.ts
│   │   ├── pool.ts                   # Pool interaction methods
│   │   ├── trading.ts                # Trading methods
│   │   ├── position.ts               # Position management
│   │   ├── collateral.ts             # Collateral utilities
│   │   └── market.ts                 # Serum market interactions
│   ├── package.json
│   └── tsconfig.json
├── tests/                            # Integration tests
│   ├── lending.ts                    # Test lending/borrowing
│   ├── trading.ts                    # Test trading
│   ├── cross-collateral.ts           # Test cross-collateralization
│   └── liquidation.ts                # Test liquidation scenarios
├── scripts/                          # Deployment and utility scripts
├── Anchor.toml                       # Anchor configuration
├── migrations/                       # Deployment migrations
└── package.json                      # Project dependencies
```

## 2. Core Program Modules

### 2.1 Pool Management Module
- **Purpose**: Manages liquidity pools for different assets
- **Key Functions**:
  - Initialize new asset pools
  - Track total deposits and borrows
  - Calculate utilization rates
  - Update interest rates based on utilization

### 2.2 Lending/Borrowing Module
- **Purpose**: Handles user deposits, withdrawals, borrows and repayments
- **Key Functions**:
  - Deposit assets to pools
  - Withdraw assets from pools
  - Borrow assets against collateral
  - Repay borrowed assets

### 2.3 Collateral Management Module
- **Purpose**: Implements cross-collateralization logic
- **Key Functions**:
  - Calculate collateral value for a user's portfolio
  - Track collateral utilization
  - Verify borrowing capacity
  - Handle collateral rebalancing

### 2.4 Trading Module
- **Purpose**: Integrates with Serum DEX for on-chain trading
- **Key Functions**:
  - Place orders on Serum orderbook
  - Calculate available margin for leverage trading
  - Settle trades
  - Update user positions

### 2.5 Yield Generation Module
- **Purpose**: Manages yield distribution to lenders
- **Key Functions**:
  - Calculate accrued interest
  - Distribute yield to lenders
  - Track yield metrics
  - Handle yield compounding

### 2.6 Liquidation Module
- **Purpose**: Monitors positions and executes liquidations when necessary
- **Key Functions**:
  - Monitor health factors
  - Trigger liquidation process
  - Reward liquidators
  - Handle collateral auctions

## 3. Key State Accounts

### 3.1 Pool State
```rust
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
```

### 3.2 User Position State
```rust
#[account]
pub struct UserPosition {
    pub owner: Pubkey,                              // User wallet
    pub collaterals: Vec<CollateralPosition>,       // User collaterals
    pub borrows: Vec<BorrowPosition>,               // User borrows
    pub health_factor: u64,                         // Current health factor
    pub last_updated: i64,                          // Last update timestamp
    pub bump: u8,                                   // PDA bump
}

pub struct CollateralPosition {
    pub pool: Pubkey,                               // Pool address
    pub amount_deposited: u64,                      // Deposited amount
    pub amount_scaled: u128,                        // Scaled amount (for yield)
    pub is_collateral: bool,                        // Used as collateral
}

pub struct BorrowPosition {
    pub pool: Pubkey,                               // Pool address
    pub amount_borrowed: u64,                       // Borrowed amount
    pub amount_scaled: u128,                        // Scaled amount (for interest)
    pub interest_rate: u64,                         // Interest rate at time of borrow
}
```

### 3.3 Market Integration State
```rust
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
```

## 4. Key Instructions

### 4.1 Initialize Pool
- Create a new lending/borrowing pool for an asset
- Set initial parameters like interest rates, LTV ratios

### 4.2 Deposit
- Deposit assets into a pool
- Receive tokenized deposit receipts
- Option to mark deposit as collateral

### 4.3 Withdraw
- Withdraw assets from a pool
- Burn tokenized deposit receipts
- Verify withdrawal doesn't breach collateral requirements

### 4.4 Borrow
- Borrow assets against deposited collateral
- Verify borrowing capacity based on cross-collateral value
- Apply interest rate based on pool utilization

### 4.5 Repay
- Repay borrowed assets
- Update user position
- Reduce debt and interest obligations

### 4.6 Trade with Leverage
- Place orders on Serum DEX with leverage
- Verify trading limits based on collateral
- Update positions after trade execution

### 4.7 Liquidate
- Liquidate undercollateralized positions
- Transfer liquidation bonus to liquidator
- Handle remaining collateral

### 4.8 Claim Yield
- Claim generated yield from lending
- Option to reinvest or withdraw yield

## 5. Frontend Structure

### 5.1 Main Views
- Dashboard - Overview of user's portfolio, health factor
- Deposit/Withdraw - Manage assets in pools
- Borrow/Repay - Borrow assets against collateral
- Trade - Trade with leverage using Serum orderbook
- Yield - View and claim generated yield

### 5.2 Key Components
- Asset Selection - Choose assets to interact with
- Risk Calculator - Calculate health factor and liquidation risks
- Leverage Simulator - Simulate leveraged trades
- Portfolio Visualizer - Visualize portfolio allocation
- Yield Tracker - Track yield generation and APY

## 6. Integration Points

### 6.1 Serum DEX Integration
- Connect to Serum orderbooks
- Place and settle orders
- Manage open orders accounts

### 6.2 Oracle Integration
- Price feeds for collateral valuation
- Risk calculation based on price movements
- Liquidation triggers based on price thresholds

### 6.3 Wallet Integration
- Connect with popular Solana wallets
- Handle transaction signing
- Manage token approvals

## 7. Getting Started

To begin implementing this MVP:

1. Set up Anchor development environment
2. Create the core program structure
3. Implement the state accounts
4. Add the primary instructions
5. Build and test the on-chain program
6. Develop the frontend client application
7. Create the TypeScript SDK
8. Write comprehensive tests