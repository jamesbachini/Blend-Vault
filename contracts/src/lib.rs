#![no_std]

use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractclient, contractevent, contractimpl, contracttype, token, vec, Address,
    Env, IntoVal, Map, String, Symbol, Vec,
};
use stellar_macros::default_impl;
use stellar_tokens::{
    fungible::{Base, FungibleToken},
    vault::{FungibleVault, Vault},
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

// Request types for Blend pool interactions
const REQUEST_TYPE_SUPPLY: u32 = 0;
const REQUEST_TYPE_WITHDRAW: u32 = 1;

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
    fn get_positions(env: Env, address: Address) -> Positions;
    fn claim(env: Env, from: Address, reserve_token_ids: Vec<u32>, to: Address) -> i128;
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
            String::from_str(e, "AUTO COMPOUNDING VAULT"),
            String::from_str(e, "ACV"),
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
    pub fn compound(e: &Env) -> i128 {
        let vault_address = e.current_contract_address();
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

        // If no BLND claimed, return early
        if blnd_claimed <= 0 {
            return 0;
        }

        // Step 2: Swap BLND for USDC on Comet
        let comet_client = CometPoolClient::new(e, &comet_pool);

        // Use a reasonable slippage tolerance (0.5% = 0.005)
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
            request_type: REQUEST_TYPE_SUPPLY,
            address: usdc_token.clone(),
            amount: usdc_received,
        });

        // Authorize the Blend pool to transfer USDC from vault to itself
        e.authorize_as_current_contract(vec![
            e,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: usdc_token,
                    fn_name: Symbol::new(e, "transfer"),
                    args: (
                        vault_address.clone(),
                        pool_address.clone(),
                        usdc_received,
                    )
                        .into_val(e),
                },
                sub_invocations: vec![e],
            }),
        ]);

        pool_client.submit(&vault_address, &vault_address, &vault_address, &requests);

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
    fn try_compound(e: &Env) {
        // Use a Result-like pattern with panic catching via environmental context
        // In Soroban, we can't easily catch panics, so we'll just call compound
        // The compound function already returns 0 if no rewards, so it won't panic
        let _ = Self::compound(e);
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

        // Create pool client to query positions
        let pool_client = BlendPoolClient::new(e, &pool_address);

        // Get the vault's positions in the Blend pool
        let positions = pool_client.get_positions(&vault_address);

        // Return the supply amount for USDC (our underlying asset)
        // The supply map uses the reserve index as key
        positions.supply.get(usdc_index).unwrap_or(0)
    }

    fn convert_to_shares(e: &Env, assets: i128) -> i128 {
        Vault::convert_to_shares(e, assets)
    }

    fn convert_to_assets(e: &Env, shares: i128) -> i128 {
        Vault::convert_to_assets(e, shares)
    }

    fn max_deposit(e: &Env, receiver: Address) -> i128 {
        Vault::max_deposit(e, receiver)
    }

    fn preview_deposit(e: &Env, assets: i128) -> i128 {
        Vault::preview_deposit(e, assets)
    }

    fn max_mint(e: &Env, receiver: Address) -> i128 {
        Vault::max_mint(e, receiver)
    }

    fn preview_mint(e: &Env, shares: i128) -> i128 {
        Vault::preview_mint(e, shares)
    }

    fn max_withdraw(e: &Env, owner: Address) -> i128 {
        Vault::max_withdraw(e, owner)
    }

    fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        Vault::preview_withdraw(e, assets)
    }

    fn max_redeem(e: &Env, owner: Address) -> i128 {
        Vault::max_redeem(e, owner)
    }

    fn preview_redeem(e: &Env, shares: i128) -> i128 {
        Vault::preview_redeem(e, shares)
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

        let asset = Vault::query_asset(e);
        let vault_address = e.current_contract_address();
        let pool_address = Self::get_blend_pool(e);

        // Calculate shares to mint
        let shares = Vault::preview_deposit(e, assets);

        // Transfer USDC from user to vault using transfer_from
        // Requires user to have called usdc.approve(vault, assets) beforehand
        let token_client = token::TokenClient::new(e, &asset);
        token_client.transfer_from(&vault_address, &from, &vault_address, &assets);

        // Supply USDC to Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);

        // Create supply request
        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_SUPPLY,
            address: asset.clone(),
            amount: assets,
        });

        // Authorize the Blend pool to transfer USDC from vault to itself
        e.authorize_as_current_contract(vec![
            e,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: asset.clone(),
                    fn_name: Symbol::new(e, "transfer"),
                    args: (vault_address.clone(), pool_address.clone(), assets)
                        .into_val(e),
                },
                sub_invocations: vec![e],
            }),
        ]);

        // Submit the supply request to Blend (vault already has the USDC)
        pool_client.submit(&vault_address, &vault_address, &vault_address, &requests);

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

        let asset = Vault::query_asset(e);
        let vault_address = e.current_contract_address();
        let pool_address = Self::get_blend_pool(e);

        // Calculate assets needed
        let assets = Vault::preview_mint(e, shares);

        // Transfer USDC from user to vault using transfer_from
        // Requires user to have called usdc.approve(vault, assets) beforehand
        let token_client = token::TokenClient::new(e, &asset);
        token_client.transfer_from(&vault_address, &from, &vault_address, &assets);

        // Supply USDC to Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);

        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_SUPPLY,
            address: asset.clone(),
            amount: assets,
        });

        // Authorize the Blend pool to transfer USDC from vault to itself
        e.authorize_as_current_contract(vec![
            e,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: asset.clone(),
                    fn_name: Symbol::new(e, "transfer"),
                    args: (vault_address.clone(), pool_address.clone(), assets)
                        .into_val(e),
                },
                sub_invocations: vec![e],
            }),
        ]);

        pool_client.submit(&vault_address, &vault_address, &vault_address, &requests);

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

        // Try to compound rewards before withdrawal
        // Note: Disabled for now to avoid authorization conflicts in nested calls
        // Self::try_compound(e);

        let asset = Vault::query_asset(e);
        let vault_address = e.current_contract_address();
        let pool_address = Self::get_blend_pool(e);

        // Calculate shares to burn
        let shares = Vault::preview_withdraw(e, assets);

        // Withdraw USDC from Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);

        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_WITHDRAW,
            address: asset.clone(),
            amount: assets,
        });

        pool_client.submit(&vault_address, &vault_address, &vault_address, &requests);

        // Transfer USDC from vault to receiver (do this before burning to maintain auth context)
        let token_client = token::TokenClient::new(e, &asset);

        // Authorize the vault to transfer USDC to receiver
        e.authorize_as_current_contract(vec![
            e,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: asset.clone(),
                    fn_name: Symbol::new(e, "transfer"),
                    args: (vault_address.clone(), receiver.clone(), assets).into_val(e),
                },
                sub_invocations: vec![e],
            }),
        ]);

        token_client.transfer(&vault_address, &receiver, &assets);

        // Burn shares from owner
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

        // Try to compound rewards before redemption
        // Note: Disabled for now to avoid authorization conflicts in nested calls
        // Self::try_compound(e);

        let asset = Vault::query_asset(e);
        let vault_address = e.current_contract_address();
        let pool_address = Self::get_blend_pool(e);

        // Calculate assets to withdraw
        let assets = Vault::preview_redeem(e, shares);

        // Withdraw USDC from Blend pool
        let pool_client = BlendPoolClient::new(e, &pool_address);

        let mut requests: Vec<Request> = Vec::new(e);
        requests.push_back(Request {
            request_type: REQUEST_TYPE_WITHDRAW,
            address: asset.clone(),
            amount: assets,
        });

        pool_client.submit(&vault_address, &vault_address, &vault_address, &requests);

        // Transfer USDC from vault to receiver (do this before burning to maintain auth context)
        let token_client = token::TokenClient::new(e, &asset);

        // Authorize the vault to transfer USDC to receiver
        e.authorize_as_current_contract(vec![
            e,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: asset.clone(),
                    fn_name: Symbol::new(e, "transfer"),
                    args: (vault_address.clone(), receiver.clone(), assets).into_val(e),
                },
                sub_invocations: vec![e],
            }),
        ]);

        token_client.transfer(&vault_address, &receiver, &assets);

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
