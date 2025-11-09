# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Soroban smart contract implementing an ERC-4626 compliant vault that deposits USDC into Blend Protocol's Yield Box Pool v2 on Stellar. Users deposit USDC, receive vault share tokens (bVLT), and earn yield from the Blend protocol.

## Build Commands

```bash
# Build for WASM target (production)
cargo build --release --target wasm32-unknown-unknown

# Build in workspace (development)
cargo build --release

# Clean build artifacts
cargo clean
```

The compiled WASM is at: `target/wasm32-unknown-unknown/release/blend_vault.wasm`

## Architecture

### Contract Structure

**Location**: `contracts/src/lib.rs`

The contract has three main implementation blocks:

1. **BlendVaultContract** - Constructor and helper functions for Blend pool integration
2. **FungibleToken** - Standard token interface (delegates to OpenZeppelin's `Vault`)
3. **FungibleVault** - ERC-4626 vault interface with custom implementations

### Key Architecture Decisions

**Blend Pool Integration Without SDK**: The contract defines the Blend pool interface directly using `#[contractclient]` instead of depending on `blend-contract-sdk`. This was necessary because:
- `blend-contract-sdk v2.22.0` depends on `soroban-sdk v22.x`
- OpenZeppelin `stellar-tokens v0.5.0` requires `soroban-sdk v23.x`
- Direct interface definition avoids version conflicts while maintaining full compatibility

The Blend pool interface is defined at lines 43-54:
```rust
#[contractclient(name = "BlendPoolClient")]
pub trait BlendPoolInterface {
    fn submit(...) -> Positions;
    fn get_positions(...) -> Positions;
}
```

**OpenZeppelin Integration**: Uses `stellar-tokens` library for:
- `Base` - Token storage and minting/burning
- `Vault` - ERC-4626 implementation (share calculations, conversions)
- `FungibleToken` and `FungibleVault` traits

The `#[default_impl]` macro generates default implementations for most vault methods, with custom overrides for:
- `total_assets()` - Queries Blend pool position instead of holding USDC directly
- `deposit()`, `mint()` - Supply USDC to Blend after receiving from user
- `withdraw()`, `redeem()` - Withdraw from Blend before returning to user

### Authorization Pattern

**The vault uses the standard SEP-41 approve/transferFrom pattern**:

- **deposit()** and **mint()** use `token_client.transfer_from(&vault, &from, &vault, &amount)`
- Users MUST call `usdc.approve(vault_address, amount)` before depositing
- This is the standard DeFi pattern (Aave, Compound, Uniswap, etc.)

**Why transfer_from, not transfer?**
- `transfer(from, to, amount)` requires `from` to authorize the transfer call
- When users call `vault.deposit()`, they only authorize the deposit call, not nested token transfers
- `transfer_from(spender, from, to, amount)` allows the vault (as approved spender) to move tokens on behalf of the user

### Data Flow

**Deposit Flow**:
1. **[USER ACTION]** User calls `usdc.approve(vault_address, amount)` to authorize vault
2. **[USER ACTION]** User calls `vault.deposit(amount, receiver, from, operator)`
3. Vault uses `transfer_from()` to pull USDC from user
4. Vault authorizes USDC transfer to Blend pool via `authorize_as_current_contract()`
5. Vault calls `blend_pool.submit()` with `REQUEST_TYPE_SUPPLY_COLLATERAL` request
6. Blend pool transfers USDC from vault to itself and records as collateral
7. Vault mints shares to receiver (`Base::mint()`)

**Withdrawal Flow**:
1. **[USER ACTION]** User calls `vault.withdraw(amount, receiver, owner, operator)`
2. Vault calls `blend_pool.submit()` with `REQUEST_TYPE_WITHDRAW_COLLATERAL` request
3. Blend pool transfers USDC from itself to vault
4. Vault burns owner's shares (`Base::burn()`)
5. Vault transfers USDC from itself to receiver

**Total Assets Calculation**:
- Queries `get_positions()` on Blend pool
- Extracts collateral amount for USDC using `usdc_reserve_index`
- Funds are deposited as collateral (not supply) which still earns interest
- Share price = total_assets / total_supply

### Storage

Contract stores two pieces of data in instance storage:
- `DataKey::BlendPool` - Address of Blend pool contract
- `DataKey::USDCReserveIndex` - Reserve index for USDC in Blend pool

## Blend Protocol Integration

**Pool Address**: `CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS`

**Request Types** (from Blend SDK):
- `REQUEST_TYPE_SUPPLY_COLLATERAL = 2` - Supply assets as collateral (used by this vault)
- `REQUEST_TYPE_WITHDRAW_COLLATERAL = 3` - Withdraw collateral assets (used by this vault)
- `REQUEST_TYPE_SUPPLY = 0` - Supply assets (non-collateralized, NOT used)
- `REQUEST_TYPE_WITHDRAW = 1` - Withdraw assets (non-collateralized, NOT used)

**Why Collateral vs Supply?**
The vault uses SupplyCollateral (type 2) instead of Supply (type 0) because:
- Both earn the same interest rate from Blend
- Collateral provides flexibility to borrow against deposits if needed in the future
- Funds appear in `positions.collateral` map instead of `positions.supply` map
- This is the standard pattern used by other Blend yield strategies

**Critical Types** (must match Blend protocol exactly):
- `Request` - Contains request_type, address, amount
- `Positions` - Contains collateral, liabilities, supply maps

## Dependency Versions

**Important**: These versions are pinned to avoid conflicts. Do NOT change:
- `soroban-sdk = "23.1.0"`
- `stellar-tokens = "0.5.0"`
- `stellar-macros = "0.5.0"`
- Other OpenZeppelin crates at `0.5.0`

Do NOT add `blend-contract-sdk` as it uses incompatible soroban-sdk v22.

## Deployment

1. Build WASM (see commands above)
2. Deploy: `stellar contract deploy --wasm target/wasm32-unknown-unknown/release/blend_vault.wasm --network mainnet`
3. Initialize with:
   - `asset` - USDC token address
   - `decimals_offset` - Use `0` for same decimals as USDC
   - `blend_pool` - Blend pool address (see above)
   - `usdc_reserve_index` - Check Blend pool's reserve configuration

## Code Patterns

**Soroban Contract Basics**:
- All contracts must start with `#![no_std]`
- Use `#[contract]` for contract struct
- Use `#[contractimpl]` for implementations
- Use `#[contracttype]` for custom types

**Working with OpenZeppelin**:
- `Base::mint(e, to, amount)` - Pass amount by value, not reference
- `Base::burn(e, from, amount)` - Pass amount by value, not reference
- `Vault::preview_*()` - Use for share/asset conversions
- Storage managed by OpenZeppelin, accessed via helper functions

**Token Authorization Patterns**:
- Use `transfer_from()` to pull tokens from users (requires prior approval)
- Use `authorize_as_current_contract()` to authorize nested contract calls
- Example authorization for Blend pool token transfer:
  ```rust
  e.authorize_as_current_contract(vec![
      e,
      InvokerContractAuthEntry::Contract(SubContractInvocation {
          context: ContractContext {
              contract: usdc_token,
              fn_name: Symbol::new(e, "transfer"),
              args: (vault_address, pool_address, amount).into_val(e),
          },
          sub_invocations: vec![e],
      }),
  ]);
  ```

**Blend Pool Interaction**:
- Create client: `BlendPoolClient::new(e, &pool_address)`
- Build requests vector with `Request` structs
- Call `submit()` with vault address as from/spender/to for vault operations
- Use `get_positions()` to query current supply/liability/collateral

## Frontend Integration

**Required User Actions for Deposits**:

```typescript
// Step 1: Approve vault to spend USDC (one-time or per-deposit)
const usdcContract = new Contract(USDC_ADDRESS);
await usdcContract.approve({
  from: userAddress,
  spender: VAULT_ADDRESS,
  amount: depositAmount,
  expiration_ledger: currentLedger + 100000 // ~5.7 days
});

// Step 2: Deposit USDC into vault
const vaultContract = new Contract(VAULT_ADDRESS);
await vaultContract.deposit({
  assets: depositAmount,
  receiver: userAddress,
  from: userAddress,
  operator: userAddress
});
```

**One-Time Approval Pattern** (recommended for better UX):
```typescript
// Approve large amount once
await usdcContract.approve({
  from: userAddress,
  spender: VAULT_ADDRESS,
  amount: 9_007_199_254_740_991n, // Max safe integer
  expiration_ledger: currentLedger + 5_256_000 // ~1 year
});

// Then deposits don't need separate approval step
await vaultContract.deposit({ ... });
```

**Withdrawal** (no approval needed):
```typescript
// Recommended: Use withdraw() with USDC amount
await vaultContract.withdraw({
  assets: withdrawAmount, // USDC amount to withdraw
  receiver: userAddress,
  owner: userAddress,
  operator: userAddress
});

// Alternative: Use redeem() with share amount (not recommended for UI)
// Only use if you need to redeem a specific number of shares
await vaultContract.redeem({
  shares: shareAmount,
  receiver: userAddress,
  owner: userAddress,
  operator: userAddress
});
```

**Best Practice**: Always use `withdraw()` with asset amounts in the frontend rather than `redeem()` with share amounts. This provides better UX as users think in terms of USDC, not shares. The contract handles all share calculations internally.

## Testing Considerations

When adding tests, note:
- Use `testutils` feature: `soroban-sdk = { version = "23.1.0", features = ["testutils"] }`
- Always call `env.mock_all_auths()` to simulate signed transactions
- Call `usdc_client.approve(&user, &vault, &amount, &ledger)` before deposits
- Mock Blend pool contract or use BlendFixture from blend-contract-sdk tests
- USDC uses 7 decimals on Stellar
- Test share price changes as yield accrues in Blend pool

**IMPORTANT** Tests should be run with the target set as x86_64-unknown-linux-gnu
cargo test --target x86_64-unknown-linux-gnu

**Example Test Pattern**:
```rust
let fixture = TestFixture::new();

// Approve vault to spend user's USDC
fixture.usdc_client.approve(&user, &vault, &i128::MAX, &200);

// Now deposits will work
let shares = fixture.vault_client.deposit(&amount, &user, &user, &user);
```
