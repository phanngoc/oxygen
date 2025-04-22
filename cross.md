Great topic — you're referring to a **cross-collateralization** model like what Oxygen (or similar DeFi platforms) offers. Let's break down the **math and mechanics** behind **multiple uses of the same collateral** to generate yield and borrow assets simultaneously.

---

### 🧠 Conceptual Summary

**Collateral reuse** (or **rehypothecation** in traditional finance) allows a user to:
- Deposit assets (e.g., SOL, BTC) into a protocol
- Lend them out to earn interest
- Simultaneously **borrow other assets** against the value of that collateral

---

### ⚙️ Mathematical Model

Let’s denote:

- \( C \): Value of the collateral deposited (in USD)
- \( LVR \): Loan-to-Value Ratio (e.g., 0.7 means you can borrow up to 70% of your collateral value)
- \( R_L \): Lending interest rate (APR you earn by lending out your assets)
- \( R_B \): Borrow interest rate (APR you pay to borrow assets)
- \( Y \): Net Yield from the operation
- \( B \): Amount borrowed (up to \( C \times LVR \))

---

### 🧮 Yield Calculation

Assume:

- You deposit $1000 worth of SOL
- LVR = 70%
- Lending rate (R_L) = 6% APR
- Borrow rate (R_B) = 3% APR

#### Step 1: Lending Side
You lend your SOL to earn interest:
- Yield from lending:  
  \[
  Y_{\text{lend}} = C \times R_L = 1000 \times 0.06 = \$60/year
  \]

#### Step 2: Borrowing Side
You borrow up to 70% of your collateral ($700) and use that borrowed asset to do something else (like staking, yield farming, or even looping back):

- Cost of borrowing:  
  \[
  C_{\text{borrow}} = B \times R_B = 700 \times 0.03 = \$21/year
  \]

#### Step 3: Net Yield
If you only consider the first level:
\[
Y = Y_{\text{lend}} - C_{\text{borrow}} = 60 - 21 = \$39/year
\]

---

### 🔁 Leverage Loop (Optional)

You can **loop** the borrowed funds back in as collateral to increase exposure:

1. Deposit $1000 → borrow $700
2. Deposit $700 → borrow $490 (LVR = 70%)
3. Deposit $490 → borrow $343...
4. Keep going until diminishing returns or liquidation risk increases

This is a **geometric series**:

- Total deposit \( D \) from infinite loop:  
  \[
  D = C \cdot \sum_{k=0}^{\infty} (LVR)^k = \frac{C}{1 - LVR} = \frac{1000}{1 - 0.7} = \$3333.33
  \]

You can now **earn 6% on $3333** while only initially supplying $1000 — but **you also owe 3% interest on about $2333 borrowed**.

---

### ⚠️ Risk Factors

- **Liquidation**: If the collateral drops in value, you're at risk.
- **Interest Rate Changes**: If borrow rate exceeds lend rate, you go negative.
- **Systemic risk**: Cascading liquidations can occur in loops.

---

### ✅ Summary

Oxygen’s model of using the same asset for **both lending and borrowing** relies on:
- Collateral value \( C \)
- LVR setting
- Rate differential between lending and borrowing
- (Optional) leverage stacking via recursive rehypothecation

The **yield math** becomes a balance of:

\[
\text{Total Yield} = \text{Lending Interest Earned} - \text{Borrowing Interest Paid}
\]

Want a Python script or spreadsheet to simulate this dynamically?