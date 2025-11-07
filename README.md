# Blend Vault - Stellar Yield Strategy

A Soroban smart contract that implements an ERC-4626 compliant vault for depositing USDC into Blend Protocol's Yield Box Pool v2 on Stellar.

## Test & Build Commands

```bash
cargo test --target x86_64-unknown-linux-gnu
cargo build --release --target wasm32-unknown-unknown
```

## Overview

This vault contract allows users to:
- Deposit USDC and receive vault share tokens (bVLT)
- Automatically supply deposited USDC to Blend Protocol's pool for yield generation
- Withdraw USDC plus accrued yield by redeeming vault shares
- Track their proportional share of the growing pool

## Architecture

The contract uses the OpenZeppelin Stellar Tokens library to implement a standard ERC-4626 vault with custom hooks to:
1. Supply deposited assets to Blend Protocol's pool
2. Query the vault's position in the Blend pool to calculate total assets
3. Withdraw from Blend when users redeem shares

### Key Components

- **Vault Token (bVLT)**: ERC-4626 compliant share token representing user's portion of the vault
- **Underlying Asset**: USDC token on Stellar
- **Yield Source**: Blend Protocol Yield Box Pool v2

## Contract Interface

### Constructor

```rust
pub fn __constructor(
    e: &Env,
    asset: Address,              // USDC token address
    decimals_offset: u32,        // Decimal offset for shares (recommended: 0)
    blend_pool: Address,         // Blend pool contract address
    usdc_reserve_index: u32,     // Reserve index for USDC in Blend pool
)
```

**Parameters:**
- `asset`: The USDC token contract address on Stellar
- `decimals_offset`: Offset for share token decimals (use 0 for same decimals as USDC)
- `blend_pool`: The Blend Protocol pool contract address (`CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS`)
- `usdc_reserve_index`: The reserve index for USDC in the Blend pool (check pool configuration)

### User Functions

#### Deposit

```rust
fn deposit(
    e: &Env,
    assets: i128,        // Amount of USDC to deposit
    receiver: Address,   // Address to receive vault shares
    from: Address,       // Address providing the USDC
    operator: Address,   // Address authorized to perform the deposit
) -> i128               // Returns: shares minted
```

Deposits USDC into the vault, supplies it to Blend, and mints vault shares to the receiver.

#### Withdraw

```rust
fn withdraw(
    e: &Env,
    assets: i128,        // Amount of USDC to withdraw
    receiver: Address,   // Address to receive the USDC
    owner: Address,      // Owner of the shares being burned
    operator: Address,   // Address authorized to perform the withdrawal
) -> i128               // Returns: shares burned
```

Withdraws USDC from Blend, burns vault shares, and transfers USDC to the receiver.

#### Redeem

```rust
fn redeem(
    e: &Env,
    shares: i128,        // Amount of vault shares to redeem
    receiver: Address,   // Address to receive the USDC
    owner: Address,      // Owner of the shares being burned
    operator: Address,   // Address authorized to perform the redemption
) -> i128               // Returns: assets withdrawn
```

Similar to withdraw, but specifies shares instead of assets.

### Query Functions

- `query_asset()`: Returns the underlying asset (USDC) address
- `total_assets()`: Returns total USDC in the vault (queries Blend pool position)
- `convert_to_shares(assets)`: Calculate shares for a given asset amount
- `convert_to_assets(shares)`: Calculate assets for a given share amount
- `preview_deposit(assets)`: Preview shares to be minted for a deposit
- `preview_withdraw(assets)`: Preview shares to be burned for a withdrawal
- `preview_mint(shares)`: Preview assets needed to mint specific shares
- `preview_redeem(shares)`: Preview assets to be received for redeeming shares

## Building

```bash
cargo build --release --target wasm32-unknown-unknown
```

The compiled WASM file will be at:
```
target/wasm32-unknown-unknown/release/blend_vault.wasm
```

## Deploying

1. Build the contract (see above)
2. Deploy using Stellar CLI:

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/blend_vault.wasm \
  --network mainnet
```

3. Initialize the contract:

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network mainnet \
  -- \
  __constructor \
  --asset <USDC_TOKEN_ADDRESS> \
  --decimals_offset 0 \
  --blend_pool CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS \
  --usdc_reserve_index <RESERVE_INDEX>
```

## Usage Example

### Depositing USDC

```bash
# Approve vault to spend your USDC
stellar contract invoke \
  --id <USDC_TOKEN_ADDRESS> \
  --network mainnet \
  -- \
  approve \
  --from <YOUR_ADDRESS> \
  --spender <VAULT_CONTRACT_ADDRESS> \
  --amount 1000000000  # 100 USDC (7 decimals)

# Deposit into vault
stellar contract invoke \
  --id <VAULT_CONTRACT_ADDRESS> \
  --network mainnet \
  -- \
  deposit \
  --assets 1000000000 \
  --receiver <YOUR_ADDRESS> \
  --from <YOUR_ADDRESS> \
  --operator <YOUR_ADDRESS>
```

### Withdrawing USDC

```bash
stellar contract invoke \
  --id <VAULT_CONTRACT_ADDRESS> \
  --network mainnet \
  -- \
  withdraw \
  --assets 1000000000 \
  --receiver <YOUR_ADDRESS> \
  --owner <YOUR_ADDRESS> \
  --operator <YOUR_ADDRESS>
```

## How It Works

1. **Deposit Flow**:
   - User approves vault to spend their USDC
   - User calls `deposit()` with USDC amount
   - Vault transfers USDC from user
   - Vault supplies USDC to Blend Protocol pool
   - Vault mints and transfers share tokens to user

2. **Yield Accrual**:
   - USDC supplied to Blend earns interest over time
   - Total vault assets increase as Blend position grows
   - Share price increases relative to underlying USDC

3. **Withdrawal Flow**:
   - User calls `withdraw()` or `redeem()`
   - Vault calculates shares to burn based on current share price
   - Vault withdraws USDC from Blend pool
   - Vault burns user's shares
   - Vault transfers USDC (including yield) to user

## Security Considerations

- The vault contract does not hold custody of private keys
- All interactions with Blend Protocol are non-custodial
- Users maintain control of their vault shares
- Vault inherits security properties of OpenZeppelin implementations
- Vault is subject to the same risks as direct Blend Protocol interaction

## Dependencies

- `soroban-sdk = "23.1.0"` - Stellar Soroban SDK
- `stellar-tokens = "0.5.0"` - OpenZeppelin token standards
- `stellar-macros = "0.5.0"` - Helper macros
- `stellar-access = "0.5.0"` - Access control utilities
- `stellar-contract-utils = "0.5.0"` - Contract utilities
- `sep-41-token = "1.3.1"` - SEP-41 token interface

## Blend Protocol Integration

This vault integrates with Blend Protocol Yield Box Pool v2:
- **Pool Address**: `CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS`
- **Dashboard**: https://mainnet.blend.capital/dashboard/?poolId=CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS

The vault uses Blend's `submit()` function to supply and withdraw assets, with request types:
- `REQUEST_TYPE_SUPPLY = 0` - Supply assets to the pool
- `REQUEST_TYPE_WITHDRAW = 1` - Withdraw assets from the pool

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please open an issue or pull request.
