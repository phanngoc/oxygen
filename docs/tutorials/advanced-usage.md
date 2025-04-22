# Oxygen Protocol: Advanced Usage Tutorial

This advanced tutorial covers complex operations within the Oxygen Protocol, including yield generation strategies, leveraged trading, and cross-collateralization benefits.

## Prerequisites

- Completion of the [Basic Usage Tutorial](./basic-usage.md)
- Understanding of DeFi concepts and terminology
- Familiarity with the Solana ecosystem

## 1. Yield Generation Strategies

The Oxygen Protocol allows you to earn yield on your deposits through various mechanisms.

### Lending Yield

When you deposit assets and enable lending, you earn interest from borrowers:

```javascript
// Deposit with lending enabled
const txSignature = await oxygenClient.deposit(
  poolAddress,
  tokenMint,
  depositAmount,
  true,  // use as collateral
  true   // enable lending
);
```

### Claiming Yield

To claim your earned yield:

```javascript
// Claim yield from a specific pool
const txSignature = await oxygenClient.claimYield(
  poolAddress,
  tokenMint
);

console.log(`Yield claimed! Transaction: ${txSignature}`);
```

### Yield Optimization

For advanced users, you can optimize your yield by:

1. Monitoring utilization rates across pools
2. Moving assets to higher-yield pools when significant differentials exist
3. Using multiple asset types to diversify your yield sources

## 2. Leveraged Trading

The Oxygen Protocol enables trading with leverage by using your deposited assets as collateral.

### Opening a Leveraged Position

```javascript
// Define the market and parameters
const marketAddress = new PublicKey('market_address_here');
const baseTokenMint = new PublicKey('base_token_mint_here');
const quoteTokenMint = new PublicKey('quote_token_mint_here');

// Trading parameters
const leverage = 3.0;  // 3x leverage
const size = new BN(1 * 10**9);  // Size in base currency
const price = new BN(100 * 10**6);  // Limit price in quote currency per base
const side = 'buy';  // 'buy' or 'sell'

// Open the leveraged position
const txSignature = await oxygenClient.openLeveragedPosition(
  marketAddress,
  baseTokenMint,
  quoteTokenMint,
  leverage,
  size,
  price,
  side
);

console.log(`Position opened! Transaction: ${txSignature}`);
```

### Managing Risk

With leveraged positions, managing risk becomes critical:

1. Set up stop-loss orders to limit potential losses
2. Monitor price movements closely
3. Maintain a healthy collateral buffer

```javascript
// Example: Setting up a stop-loss
const stopLossPrice = new BN(95 * 10**6);  // 5% below entry for a long position

const txSignature = await oxygenClient.setStopLoss(
  positionId,
  stopLossPrice
);
```

### Closing a Leveraged Position

```javascript
// Close an existing position
const txSignature = await oxygenClient.closeLeveragedPosition(
  positionId
);

console.log(`Position closed! Transaction: ${txSignature}`);
```

## 3. Cross-Collateralization Benefits

One of Oxygen's most powerful features is cross-collateralization, which enables you to use your entire portfolio as collateral.

### Optimizing Collateral Usage

```javascript
// Get your current collateral usage efficiency
const collateralEfficiency = await oxygenClient.getCollateralEfficiency();

console.log('Collateral efficiency:', collateralEfficiency);
// Higher percentage means better capital efficiency
```

### Managing Multiple Collateral Types

With cross-collateralization, you can:

1. Deposit multiple asset types as collateral
2. Borrow against your entire portfolio
3. Reduce liquidation risks through diversification

```javascript
// Optimize collateral allocation based on current market conditions
const recommendation = await oxygenClient.getOptimalCollateralAllocation();

console.log('Recommended collateral allocation:', recommendation);
// This provides suggested adjustments to minimize liquidation risk
```

## 4. Advanced Liquidation Prevention

To avoid liquidation of your positions:

```javascript
// Get warning if your position is approaching liquidation territory
const healthWarnings = await oxygenClient.getHealthWarnings();

if (healthWarnings.length > 0) {
  console.log('Warning: Position at risk of liquidation');
  
  // Get recommendations to improve your health factor
  const recommendations = await oxygenClient.getHealthImprovementOptions();
  console.log('Recommended actions:', recommendations);
}
```

## 5. Protocol Governance Participation

As an advanced user, you can participate in protocol governance:

```javascript
// Get active governance proposals
const proposals = await oxygenClient.getActiveProposals();

// Vote on a proposal
const proposalId = proposals[0].id;
const vote = 'approve';  // 'approve' or 'reject'

const txSignature = await oxygenClient.castVote(
  proposalId,
  vote
);
```

## 6. Analytics and Portfolio Management

Access advanced analytics for your positions:

```javascript
// Get comprehensive portfolio analytics
const analytics = await oxygenClient.getPortfolioAnalytics();

console.log('Current APY:', analytics.totalApy);
console.log('Portfolio allocation:', analytics.allocation);
console.log('Risk metrics:', analytics.riskMetrics);
```

## Best Practices

1. **Risk Management**: Never borrow more than 70% of your maximum borrowing capacity
2. **Diversification**: Spread your collateral across multiple asset types
3. **Monitor Regularly**: Check your health factor and positions frequently, especially during market volatility
4. **Gas Optimization**: Batch transactions when possible to save on fees
5. **Security**: Use hardware wallets for large positions and enable all security features

## Next Steps

- [Protocol Architecture](../architecture/overview.md): Understand the technical design
- [Advanced API Features](../api/advanced.md): Learn about advanced SDK capabilities
- [Code Examples Repository](https://github.com/oxygen-protocol/examples): Real-world implementation examples