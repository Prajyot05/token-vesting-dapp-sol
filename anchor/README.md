# Anchor Vesting Program

This template includes a simple SOL vesting program built with [Anchor](https://www.anchor-lang.com/).

## Pre-deployed Program

The vesting program is deployed on **devnet** at:

```
F4jZpgbtTb6RWNWq6v35fUeiAsRJMrDczVPv9U23yXjB
```

You can interact with it immediately by connecting your wallet to devnet.

## Deploying Your Own Program

To deploy your own version of the program:

### 1. Generate a new program keypair

```bash
cd anchor
solana-keygen new -o target/deploy/vesting-keypair.json
```

### 2. Get the new program ID

```bash
solana address -k target/deploy/vesting-keypair.json
```

### 3. Update the program ID

Update the program ID in these files:

- `anchor/Anchor.toml` - Update `vesting = "..."` under `[programs.devnet]`
- `anchor/programs/vesting/src/lib.rs` - Update `declare_id!("...")`

### 4. Build and deploy

```bash
# Build the program
anchor build

# Get devnet SOL for deployment (~2 SOL needed)
solana airdrop 2 --url devnet

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

### 5. Regenerate the TypeScript client

```bash
cd ..
npm run codama:js
```

This updates the generated client code in `app/generated/vesting/` with your new program ID.

## Program Overview

The vesting program allows users to:

- **Deposit**: Send SOL to a personal vesting PDA (Program Derived Address)
- **Withdraw**: Retrieve all SOL from your vesting

Each user gets their own vesting derived from their wallet address.

## Testing

Run the Anchor tests:

```bash
anchor test --skip-deploy
```
