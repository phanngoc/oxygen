# Installation Guide

This guide will help you set up your development environment for working with the Oxygen Protocol.

## Prerequisites

Before starting, ensure you have the following installed:

- [Node.js](https://nodejs.org/) (v14 or later)
- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Solana CLI Tools](https://docs.solana.com/cli/install-solana-cli-tools) (v1.10 or later)
- [Anchor Framework](https://project-serum.github.io/anchor/getting-started/installation.html) (v0.24.2 or later)
- [Git](https://git-scm.com/downloads)

## Step 1: Clone the Repository

```bash
git clone https://github.com/oxygen-protocol/oxygen.git
cd oxygen
```

## Step 2: Install Dependencies

```bash
# Install JavaScript dependencies
npm install

# Build the Anchor program
cd app/programs/oxygen
anchor build
```

## Step 3: Configure Your Solana Environment

Ensure your Solana CLI is configured for development:

```bash
solana config set --url devnet  # Or localhost for local development
solana-keygen new -o ~/.config/solana/id.json  # Only if you need a new keypair
```

## Step 4: Deploy the Program (Development)

For local development with a local validator:

```bash
# Start a local Solana validator
solana-test-validator

# In a new terminal, deploy the program
cd app
anchor deploy
```

For deploying to devnet:

```bash
cd app
anchor deploy --provider.cluster devnet
```

## Step 5: Run Tests

```bash
# Run the test suite
npm test
```

## Step 6: Setup SDK

```bash
cd sdk
npm install
npm run build
```

## Local Development

For local development and testing:

1. Start a local validator:
```bash
solana-test-validator
```

2. In a new terminal, run the development environment:
```bash
npm run dev
```

This will start a local development server with hot-reloading enabled.

## Troubleshooting

### Common Issues

1. **Build errors with Anchor**:
   - Ensure you're using the correct version of Anchor (v0.24.2+)
   - Try `anchor clean` before rebuilding

2. **Connection issues with Solana**:
   - Check your network connection
   - Verify your RPC URL with `solana config get`

3. **Deployment failures**:
   - Ensure you have sufficient SOL for deployment
   - Check for program size limitations

For more support, join our Discord community or open an issue on GitHub.

## Next Steps

After setting up your development environment, check out these resources:

- [Basic Tutorial](../tutorials/basic-usage.md) - Learn to use the protocol
- [Architecture Overview](../architecture/overview.md) - Understand the protocol design
- [SDK Documentation](../api/sdk.md) - Learn to interact with the protocol programmatically