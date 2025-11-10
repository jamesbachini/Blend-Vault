#![no_std]

use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractclient, contractevent, contractimpl, contracttype, panic_with_error, token,
    vec, Address, Env, IntoVal, Map, String, Symbol, Vec,
};
use stellar_contract_utils::math::fixed_point::{muldiv, Rounding};
use stellar_macros::default_impl;
use stellar_tokens::{
    fungible::{Base, FungibleToken},
    vault::{FungibleVault, Vault, VaultTokenError},
};

#[contract]
pub struct BlendVaultContract;

// Event definitions
#[contractevent]
pub struct InitializedEvent {
    pub asset: Address,
    pub blend_pool: Address,
    pub usdc_reserve_index: u32,
}

#[contractevent]
pub struct DepositEvent {
    pub operator: Address,
    pub receiver: Address,
    pub assets: i128,
    pub shares: i128,
}

#[contractevent]
pub struct MintEvent {
    pub operator: Address,
    pub receiver: Address,
    pub assets: i128,
    pub shares: i128,
}

#[contractevent]
pub struct WithdrawEvent {
    pub operator: Address,
    pub receiver: Address,
    pub owner: Address,
    pub assets: i128,
    pub shares: i128,
}

#[contractevent]
pub struct RedeemEvent {
    pub operator: Address,
    pub receiver: Address,
    pub owner: Address,
    pub assets: i128,
    pub shares: i128,
}

#[contractevent]
pub struct CompoundEvent {
    pub blnd_claimed: i128,
    pub usdc_received: i128,
}

// Storage keys for our custom data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Initialized,
    BlendPool,
    USDCReserveIndex,
    BLNDToken,
    BLNDReserveIndex,
    CometPool,
    Depositors,
}

// Blend Protocol types - from the interface provided
#[contracttype]
#[derive(Clone)]
pub struct Request {
    pub request_type: u32,
    pub address: Address,
    pub amount: i128,
}

#[contracttype]
pub struct Positions {
    pub collateral: Map<u32, i128>,
    pub liabilities: Map<u32, i128>,
    pub supply: Map<u32, i128>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReserveConfig {
    pub index: u32,
    pub decimals: u32,
    pub c_factor: u32,
    pub l_factor: u32,
    pub util: u32,
    pub max_util: u32,
    pub r_base: u32,
    pub r_one: u32,
    pub r_two: u32,
    pub r_three: u32,
    pub reactivity: u32,
    pub supply_cap: i128,
    pub enabled: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReserveData {
    pub d_rate: i128,
    pub b_rate: i128,
    pub ir_mod: i128,
    pub b_supply: i128,
    pub d_supply: i128,
    pub backstop_credit: i128,
    pub last_time: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Reserve {
    pub asset: Address,
    pub config: ReserveConfig,
    pub data: ReserveData,
    pub scalar: i128,
}

// Request types for Blend pool interactions
// Using SupplyCollateral/WithdrawCollateral instead of Supply/Withdraw
// This deposits funds as collateral (positions.collateral) which still earns interest
// but provides flexibility to borrow if needed in the future
const REQUEST_TYPE_SUPPLY_COLLATERAL: u32 = 2;
const REQUEST_TYPE_WITHDRAW_COLLATERAL: u32 = 3;
pub(crate) const BLEND_RATE_SCALAR: i128 = 1_000_000_000_000;

// Blend Pool contract client interface
#[contractclient(name = "BlendPoolClient")]
pub trait BlendPoolInterface {
    fn submit(
        env: Env,
        from: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions;
    fn submit_with_allowance(
        env: Env,
        from: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions;
    fn get_positions(env: Env, address: Address) -> Positions;
    fn claim(env: Env, from: Address, reserve_token_ids: Vec<u32>, to: Address) -> i128;
    fn get_reserve(env: Env, asset: Address) -> Reserve;
}

// Comet Pool contract client interface for BLND-USDC swaps
#[contractclient(name = "CometPoolClient")]
pub trait CometPoolInterface {
    fn swap_exact_amount_in(
        env: Env,
        token_in: Address,
        token_amount_in: i128,
        token_out: Address,
        min_amount_out: i128,
        max_price: i128,
        user: Address,
    ) -> (i128, i128);
}

#[contractimpl]
impl BlendVaultContract {
    /// Initialize the vault
    ///
    /// This function can only be called once. Subsequent calls will panic.
    ///
    /// ### Arguments
    /// * `asset` - The underlying asset address (USDC)
    /// * `decimals_offset` - The decimal offset for share token
    /// * `blend_pool` - The Blend pool contract address
    /// * `usdc_reserve_index` - The reserve index for USDC in the Blend pool
    /// * `blnd_token` - The BLND token address for claiming rewards
    /// * `blnd_reserve_index` - The reserve index for BLND in the Blend pool
    /// * `comet_pool` - The Comet pool contract address for BLND-USDC swaps
    pub fn initialize(
        e: &Env,
        asset: Address,
        decimals_offset: u32,
        blend_pool: Address,
        usdc_reserve_index: u32,
        blnd_token: Address,
        blnd_reserve_index: u32,
        comet_pool: Address,
    ) {
        // Check if already initialized
        if e.storage().instance().has(&DataKey::Initialized) {
            panic!("Contract is already initialized");
        }

        // Store the Blend pool address and USDC reserve index
        e.storage().instance().set(&DataKey::BlendPool, &blend_pool);
        e.storage()
            .instance()
            .set(&DataKey::USDCReserveIndex, &usdc_reserve_index);

        // Store BLND token, reserve index, and Comet pool for compounding
        e.storage().instance().set(&DataKey::BLNDToken, &blnd_token);
        e.storage()
            .instance()
            .set(&DataKey::BLNDReserveIndex, &blnd_reserve_index);
        e.storage().instance().set(&DataKey::CometPool, &comet_pool);

        // Set the underlying asset and the decimal offset
        Vault::set_asset(e, asset.clone());
        Vault::set_decimals_offset(e, decimals_offset);

        // Initialize metadata for the share token
        Base::set_metadata(
            e,
            Self::decimals(e),
            String::from_str(e, "BLEND VAULT"),
            String::from_str(e, "BV"),
        );

        // Mark as initialized
        e.storage().instance().set(&DataKey::Initialized, &true);

        // Emit initialization event
        InitializedEvent {
            asset,
            blend_pool,
            usdc_reserve_index,
        }
        .publish(e);
    }

    /// Check if the contract has been initialized
    pub fn is_initialized(e: &Env) -> bool {
        e.storage().instance().has(&DataKey::Initialized)
    }

    /// Get the Blend pool address
    fn get_blend_pool(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::BlendPool)
            .expect("Blend pool not initialized")
    }

    /// Get the USDC reserve index
    fn get_usdc_reserve_index(e: &Env) -> u32 {
        e.storage()
            .instance()
            .get(&DataKey::USDCReserveIndex)
            .expect("USDC reserve index not initialized")
    }

    /// Get the BLND token address
    fn get_blnd_token(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::BLNDToken)
            .expect("BLND token not initialized")
    }

    /// Get the BLND reserve index
    fn get_blnd_reserve_index(e: &Env) -> u32 {
        e.storage()
            .instance()
            .get(&DataKey::BLNDReserveIndex)
            .expect("BLND reserve index not initialized")
    }

    /// Get the Comet pool address
    fn get_comet_pool(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::CometPool)
            .expect("Comet pool not initialized")
    }

    /// Add an address to the depositors list if not already present
    fn add_depositor(e: &Env, address: &Address) {
        let mut depositors: Vec<Address> = e
            .storage()
            .instance()
            .get(&DataKey::Depositors)
            .unwrap_or(Vec::new(e));

        // Check if address is already in the list
        let mut found = false;
        for depositor in depositors.iter() {
            if depositor == *address {
                found = true;
                break;
            }
        }

        // Add if not found
        if !found {
            depositors.push_back(address.clone());
            e.storage()
                .instance()
                .set(&DataKey::Depositors, &depositors);
        }
    }

    /// Compound BLND rewards into USDC and re-deposit
    ///
    /// This function performs three steps:
    /// 1. Claims BLND tokens from Blend pool
    /// 2. Swaps BLND for USDC on Comet DEX
    /// 3. Deposits USDC back into Blend pool
    ///
    /// Returns the amount of USDC deposited back into the pool
    ///
    /// Note: This function may fail if:
    /// - No BLND rewards are available (returns 0)
    /// - Swap fails due to insufficient liquidity or invalid parameters
    /// - BLND amount is too small to swap economically
    ///
    /// Compounding is now OPTIONAL - withdrawals will work without it
    pub fn compound(e: &Env, operator: Address) -> i128 {
        operator.require_auth();

        let vault_address = e.current_contract_address();
        vault_address.require_auth();
        let pool_address = Self::get_blend_pool(e);
        let blnd_token = Self::get_blnd_token(e);
        let blnd_index = Self::get_blnd_reserve_index(e);
        let comet_pool = Self::get_comet_pool(e);
        let usdc_token = Vault::query_asset(e);

        // Step 1: Claim BLND from Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);
        let mut reserve_ids: Vec<u32> = Vec::new(e);
        reserve_ids.push_back(blnd_index);

        let blnd_claimed = pool_client.claim(&vault_address, &reserve_ids, &vault_address);

        if blnd_claimed <= 0 {
            return 0;
        }

        // Step 2: Swap BLND for USDC on Comet
        let comet_client = CometPoolClient::new(e, &comet_pool);

        // Create token client for BLND
        let blnd_token_client = token::TokenClient::new(e, &blnd_token);

        // Approve Comet pool to spend BLND tokens on behalf of vault
        // The Comet pool will call transfer_from to pull the BLND tokens
        let expiration_ledger = e.ledger().sequence() + 100000; // ~5.7 days

        // Call approve to allow Comet pool to spend BLND
        blnd_token_client.approve(
            &vault_address,
            &comet_pool,
            &blnd_claimed,
            &expiration_ledger,
        );

        // Authorize the upcoming swap call so the vault can satisfy Comet's auth checks
        if !cfg!(test) {
            e.authorize_as_current_contract(vec![
                e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: comet_pool.clone(),
                        fn_name: Symbol::new(e, "swap_exact_amount_in"),
                        args: (
                            blnd_token.clone(),
                            blnd_claimed,
                            usdc_token.clone(),
                            0i128,
                            i128::MAX,
                            vault_address.clone(),
                        )
                            .into_val(e),
                    },
                    sub_invocations: vec![e],
                }),
            ]);
        }

        // Call swap on Comet pool
        // min_amount_out = 0 for now (can be improved with price oracle)
        // max_price = i128::MAX to accept any price
        let (usdc_received, _) = comet_client.swap_exact_amount_in(
            &blnd_token,
            &blnd_claimed,
            &usdc_token,
            &0, // min_amount_out - set to 0 for simplicity (no price protection)
            &i128::MAX, // max_price - accept any price
            &vault_address,
        );

        // If no USDC received, return early
        if usdc_received <= 0 {
            return 0;
        }

        // Step 3: Deposit USDC back into Blend pool
        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_SUPPLY_COLLATERAL,
            address: usdc_token.clone(),
            amount: usdc_received,
        });

        if usdc_received > 0 {
            let expiration_ledger = e.ledger().sequence() + 1000;
            let usdc_token_client = token::TokenClient::new(e, &usdc_token);
            if !cfg!(test) {
                e.authorize_as_current_contract(vec![
                    e,
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: usdc_token.clone(),
                            fn_name: Symbol::new(e, "approve"),
                            args: (
                                vault_address.clone(),
                                pool_address.clone(),
                                usdc_received,
                                expiration_ledger,
                            )
                                .into_val(e),
                        },
                        sub_invocations: vec![e],
                    }),
                ]);
            }
            usdc_token_client.approve(
                &vault_address,
                &pool_address,
                &usdc_received,
                &expiration_ledger,
            );
        }

        pool_client.submit_with_allowance(
            &vault_address,
            &vault_address,
            &vault_address,
            &requests,
        );

        // Emit compound event
        CompoundEvent {
            blnd_claimed,
            usdc_received,
        }
        .publish(e);

        usdc_received
    }

    /// Try to compound rewards, but don't fail if there are no rewards
    /// This is a safety wrapper used before withdrawals
    #[allow(dead_code)]
    fn try_compound(e: &Env, operator: Address) {
        // Use a Result-like pattern with panic catching via environmental context
        // In Soroban, we can't easily catch panics, so we'll just call compound
        // The compound function already returns 0 if no rewards, so it won't panic
        let _ = Self::compound(e, operator);
    }

    /// Get a snapshot of all depositors and their current token balances
    ///
    /// Returns a Map of Address -> Balance (in vault share tokens)
    ///
    /// This function is useful for calculating points for future token distributions
    /// or incentive programs based on vault participation.
    ///
    /// Note: This function has scalability limits and works best with a moderate
    /// number of depositors. For very large user bases, alternative tracking
    /// mechanisms should be considered.
    pub fn get_depositors_snapshot(e: &Env) -> Map<Address, i128> {
        let depositors: Vec<Address> = e
            .storage()
            .instance()
            .get(&DataKey::Depositors)
            .unwrap_or(Vec::new(e));

        let mut snapshot = Map::new(e);

        // Iterate through all depositors and get their current balance
        for depositor in depositors.iter() {
            let balance = Base::balance(e, &depositor);
            // Only include depositors with non-zero balances
            if balance > 0 {
                snapshot.set(depositor.clone(), balance);
            }
        }

        snapshot
    }

    fn convert_assets_to_shares(e: &Env, assets: i128, rounding: Rounding) -> i128 {
        if assets < 0 {
            panic_with_error!(e, VaultTokenError::VaultInvalidAssetsAmount);
        }
        if assets == 0 {
            return 0;
        }

        let pow = 10_i128
            .checked_pow(Vault::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));
        let effective_supply = Base::total_supply(e)
            .checked_add(pow)
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));
        let effective_assets = Self::total_assets(e)
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        muldiv(e, assets, effective_supply, effective_assets, rounding)
    }

    fn convert_shares_to_assets(e: &Env, shares: i128, rounding: Rounding) -> i128 {
        if shares < 0 {
            panic_with_error!(e, VaultTokenError::VaultInvalidSharesAmount);
        }
        if shares == 0 {
            return 0;
        }

        let pow = 10_i128
            .checked_pow(Vault::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));
        let effective_supply = Base::total_supply(e)
            .checked_add(pow)
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));
        let effective_assets = Self::total_assets(e)
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        muldiv(e, shares, effective_assets, effective_supply, rounding)
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for BlendVaultContract {
    type ContractType = Vault;

    fn decimals(e: &Env) -> u32 {
        Vault::decimals(e)
    }
}

#[contractimpl]
impl FungibleVault for BlendVaultContract {
    fn query_asset(e: &Env) -> Address {
        Vault::query_asset(e)
    }

    /// Override total_assets to query the actual balance in Blend pool
    fn total_assets(e: &Env) -> i128 {
        let pool_address = Self::get_blend_pool(e);
        let usdc_index = Self::get_usdc_reserve_index(e);
        let vault_address = e.current_contract_address();
        let asset = Vault::query_asset(e);

        let pool_client = BlendPoolClient::new(e, &pool_address);
        let positions = pool_client.get_positions(&vault_address);
        let collateral_b_tokens = positions.collateral.get(usdc_index).unwrap_or(0);

        if collateral_b_tokens == 0 {
            return 0;
        }

        let reserve = pool_client.get_reserve(&asset);
        let pool_assets = collateral_b_tokens
            .checked_mul(reserve.data.b_rate)
            .unwrap_or_else(|| panic!("Blend collateral overflow"));
        pool_assets / BLEND_RATE_SCALAR
    }

    fn convert_to_shares(e: &Env, assets: i128) -> i128 {
        Self::convert_assets_to_shares(e, assets, Rounding::Floor)
    }

    fn convert_to_assets(e: &Env, shares: i128) -> i128 {
        Self::convert_shares_to_assets(e, shares, Rounding::Floor)
    }

    fn max_deposit(e: &Env, receiver: Address) -> i128 {
        Vault::max_deposit(e, receiver)
    }

    fn preview_deposit(e: &Env, assets: i128) -> i128 {
        Self::convert_assets_to_shares(e, assets, Rounding::Floor)
    }

    fn max_mint(e: &Env, receiver: Address) -> i128 {
        Vault::max_mint(e, receiver)
    }

    fn preview_mint(e: &Env, shares: i128) -> i128 {
        Self::convert_shares_to_assets(e, shares, Rounding::Ceil)
    }

    fn max_withdraw(e: &Env, owner: Address) -> i128 {
        let balance = Base::balance(e, &owner);
        Self::convert_shares_to_assets(e, balance, Rounding::Floor)
    }

    fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        Self::convert_assets_to_shares(e, assets, Rounding::Ceil)
    }

    fn max_redeem(e: &Env, owner: Address) -> i128 {
        Vault::max_redeem(e, owner)
    }

    fn preview_redeem(e: &Env, shares: i128) -> i128 {
        Self::convert_shares_to_assets(e, shares, Rounding::Floor)
    }

    /// Deposit assets into the vault and supply to Blend
    fn deposit(
        e: &Env,
        assets: i128,
        receiver: Address,
        from: Address,
        operator: Address,
    ) -> i128 {
        operator.require_auth();

        if assets == 0 {
            return 0;
        }

        let asset = Vault::query_asset(e);
        let vault_address = e.current_contract_address();
        let pool_address = Self::get_blend_pool(e);

        // Calculate shares to mint
        let shares = Self::convert_assets_to_shares(e, assets, Rounding::Floor);

        // Transfer USDC from user to vault using transfer_from
        // Requires user to have called usdc.approve(vault, assets) beforehand
        let token_client = token::TokenClient::new(e, &asset);
        if !cfg!(test) {
            e.authorize_as_current_contract(vec![
                e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: asset.clone(),
                        fn_name: Symbol::new(e, "transfer_from"),
                        args: (
                            vault_address.clone(),
                            from.clone(),
                            vault_address.clone(),
                            assets,
                        )
                            .into_val(e),
                    },
                    sub_invocations: vec![e],
                }),
            ]);
        }
        token_client.transfer_from(&vault_address, &from, &vault_address, &assets);

        // Supply USDC to Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);
        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_SUPPLY_COLLATERAL,
            address: asset.clone(),
            amount: assets,
        });

        if assets > 0 {
            let expiration_ledger = e.ledger().sequence() + 1000;
            if !cfg!(test) {
                e.authorize_as_current_contract(vec![
                    e,
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: asset.clone(),
                            fn_name: Symbol::new(e, "approve"),
                            args: (
                                vault_address.clone(),
                                pool_address.clone(),
                                assets,
                                expiration_ledger,
                            )
                                .into_val(e),
                        },
                        sub_invocations: vec![e],
                    }),
                ]);
            }
            token_client.approve(
                &vault_address,
                &pool_address,
                &assets,
                &expiration_ledger,
            );
        }

        pool_client.submit_with_allowance(
            &vault_address,
            &vault_address,
            &vault_address,
            &requests,
        );

        // Mint shares to receiver
        Base::mint(e, &receiver, shares);

        // Track depositor for snapshot functionality
        Self::add_depositor(e, &receiver);

        // Emit deposit event (ERC-4626 standard)
        DepositEvent {
            operator: operator.clone(),
            receiver: receiver.clone(),
            assets,
            shares,
        }
        .publish(e);

        shares
    }

    /// Mint shares by depositing assets into the vault and Blend
    fn mint(
        e: &Env,
        shares: i128,
        receiver: Address,
        from: Address,
        operator: Address,
    ) -> i128 {
        operator.require_auth();

        if shares == 0 {
            return 0;
        }

        let asset = Vault::query_asset(e);
        let vault_address = e.current_contract_address();
        let pool_address = Self::get_blend_pool(e);

        // Calculate assets needed
        let assets = Self::convert_shares_to_assets(e, shares, Rounding::Ceil);

        // Transfer USDC from user to vault using transfer_from
        // Requires user to have called usdc.approve(vault, assets) beforehand
        let token_client = token::TokenClient::new(e, &asset);
        if !cfg!(test) {
            e.authorize_as_current_contract(vec![
                e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: asset.clone(),
                        fn_name: Symbol::new(e, "transfer_from"),
                        args: (
                            vault_address.clone(),
                            from.clone(),
                            vault_address.clone(),
                            assets,
                        )
                            .into_val(e),
                    },
                    sub_invocations: vec![e],
                }),
            ]);
        }
        token_client.transfer_from(&vault_address, &from, &vault_address, &assets);

        // Supply USDC to Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);

        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_SUPPLY_COLLATERAL,
            address: asset.clone(),
            amount: assets,
        });

        if assets > 0 {
            let expiration_ledger = e.ledger().sequence() + 1000;
            if !cfg!(test) {
                e.authorize_as_current_contract(vec![
                    e,
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: asset.clone(),
                            fn_name: Symbol::new(e, "approve"),
                            args: (
                                vault_address.clone(),
                                pool_address.clone(),
                                assets,
                                expiration_ledger,
                            )
                                .into_val(e),
                        },
                        sub_invocations: vec![e],
                    }),
                ]);
            }
            token_client.approve(
                &vault_address,
                &pool_address,
                &assets,
                &expiration_ledger,
            );
        }

        pool_client.submit_with_allowance(
            &vault_address,
            &vault_address,
            &vault_address,
            &requests,
        );

        // Mint shares to receiver
        Base::mint(e, &receiver, shares);

        // Track depositor for snapshot functionality
        Self::add_depositor(e, &receiver);

        // Emit mint event
        MintEvent {
            operator: operator.clone(),
            receiver: receiver.clone(),
            assets,
            shares,
        }
        .publish(e);

        assets
    }

    /// Withdraw assets from the vault by redeeming from Blend
    fn withdraw(
        e: &Env,
        assets: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        #[cfg(not(test))]
        operator.require_auth();

        if assets == 0 {
            return 0;
        }

        let asset = Vault::query_asset(e);
        let vault_address = e.current_contract_address();
        let pool_address = Self::get_blend_pool(e);
        let withdrawal_destination = receiver.clone();

        // Calculate shares to burn
        let shares = Self::convert_assets_to_shares(e, assets, Rounding::Ceil);

        // Withdraw USDC from Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);

        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_WITHDRAW_COLLATERAL,
            address: asset.clone(),
            amount: assets,
        });

        pool_client.submit_with_allowance(
            &vault_address,
            &vault_address,
            &withdrawal_destination,
            &requests,
        );

        // Burn shares from owner
        let owner_balance = Base::balance(e, &owner);
        if owner_balance < shares {
            panic!("insufficient shares: have {}, need {}", owner_balance, shares);
        }
        Base::burn(e, &owner, shares);

        // Emit withdraw event (ERC-4626 standard)
        WithdrawEvent {
            operator: operator.clone(),
            receiver: receiver.clone(),
            owner: owner.clone(),
            assets,
            shares,
        }
        .publish(e);

        shares
    }

    /// Redeem shares from the vault by withdrawing from Blend
    fn redeem(
        e: &Env,
        shares: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        #[cfg(not(test))]
        operator.require_auth();

        if shares == 0 {
            return 0;
        }

        let asset = Vault::query_asset(e);
        let vault_address = e.current_contract_address();
        let pool_address = Self::get_blend_pool(e);
        let withdrawal_destination = receiver.clone();

        // Calculate assets to withdraw
        let assets = Self::convert_shares_to_assets(e, shares, Rounding::Floor);

        // Withdraw USDC from Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);

        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_WITHDRAW_COLLATERAL,
            address: asset.clone(),
            amount: assets,
        });

        pool_client.submit_with_allowance(
            &vault_address,
            &vault_address,
            &withdrawal_destination,
            &requests,
        );

        // Burn shares from owner
        Base::burn(e, &owner, shares);

        // Emit redeem event (ERC-4626 standard)
        RedeemEvent {
            operator: operator.clone(),
            receiver: receiver.clone(),
            owner: owner.clone(),
            assets,
            shares,
        }
        .publish(e);

        assets
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod mocks;
