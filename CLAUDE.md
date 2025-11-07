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

### Data Flow

**Deposit Flow**:
1. User approves vault to spend USDC
2. Vault transfers USDC from user (`token::TokenClient`)
3. Vault supplies USDC to Blend pool via `submit()` with `REQUEST_TYPE_SUPPLY`
4. Vault mints shares to user (`Base::mint()`)

**Withdrawal Flow**:
1. Vault withdraws USDC from Blend pool via `submit()` with `REQUEST_TYPE_WITHDRAW`
2. Vault burns user's shares (`Base::burn()`)
3. Vault transfers USDC to user

**Total Assets Calculation**:
- Queries `get_positions()` on Blend pool
- Extracts supply amount for USDC using `usdc_reserve_index`
- Share price = total_assets / total_supply

### Storage

Contract stores two pieces of data in instance storage:
- `DataKey::BlendPool` - Address of Blend pool contract
- `DataKey::USDCReserveIndex` - Reserve index for USDC in Blend pool

## Blend Protocol Integration

**Pool Address**: `CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS`

**Request Types**:
- `REQUEST_TYPE_SUPPLY = 0` - Supply assets to pool
- `REQUEST_TYPE_WITHDRAW = 1` - Withdraw assets from pool

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

**Blend Pool Interaction**:
- Create client: `BlendPoolClient::new(e, &pool_address)`
- Build requests vector with `Request` structs
- Call `submit()` with vault address as from/spender/to for vault operations
- Use `get_positions()` to query current supply/liability/collateral

## Testing Considerations

When adding tests, note:
- Use `testutils` feature: `soroban-sdk = { version = "23.1.0", features = ["testutils"] }`
- Mock Blend pool contract or use BlendFixture from blend-contract-sdk tests
- USDC uses 7 decimals on Stellar
- Test share price changes as yield accrues in Blend pool
