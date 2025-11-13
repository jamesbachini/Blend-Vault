#![cfg(test)]

use crate::{
    Positions, Request, Reserve, ReserveConfig, ReserveData, BLEND_RATE_SCALAR,
    REQUEST_TYPE_SUPPLY_COLLATERAL, REQUEST_TYPE_WITHDRAW_COLLATERAL,
};
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Map, Vec};

// Storage keys for MockBlendPool
#[contracttype]
#[derive(Clone)]
pub enum MockPoolDataKey {
    Positions(Address),
    Reserve(Address),
    RewardToken,
}

fn read_b_rate(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&MockPoolDataKey::Reserve(asset.clone()))
        .unwrap_or(BLEND_RATE_SCALAR)
}

fn store_b_rate(env: &Env, asset: &Address, b_rate: i128) {
    env.storage()
        .persistent()
        .set(&MockPoolDataKey::Reserve(asset.clone()), &b_rate);
}

fn build_reserve(asset: Address, b_rate: i128) -> Reserve {
    Reserve {
        asset: asset.clone(),
        config: ReserveConfig {
            index: 0,
            decimals: 7,
            c_factor: 0,
            l_factor: 0,
            util: 0,
            max_util: 0,
            r_base: 0,
            r_one: 0,
            r_two: 0,
            r_three: 0,
            reactivity: 0,
            supply_cap: i128::MAX,
            enabled: true,
        },
        data: ReserveData {
            d_rate: BLEND_RATE_SCALAR,
            b_rate,
            ir_mod: 0,
            b_supply: 0,
            d_supply: 0,
            backstop_credit: 0,
            last_time: 0,
        },
        scalar: 10i128.pow(7),
    }
}

// Mock Blend Pool Contract
#[contract]
pub struct MockBlendPool;

#[contractimpl]
impl MockBlendPool {
    pub fn set_reward_token(env: Env, token: Address) {
        env.storage()
            .persistent()
            .set(&MockPoolDataKey::RewardToken, &token);
    }

    pub fn submit(
        env: Env,
        owner: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        Self::process_requests(env, owner, spender, to, requests)
    }

    pub fn submit_with_allowance(
        env: Env,
        owner: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        Self::process_requests(env, owner, spender, to, requests)
    }

    pub fn set_b_rate(env: Env, asset: Address, b_rate: i128) {
        store_b_rate(&env, &asset, b_rate);
    }

    pub fn get_reserve(env: Env, asset: Address) -> Reserve {
        build_reserve(asset.clone(), read_b_rate(&env, &asset))
    }

    fn process_requests(
        env: Env,
        owner: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        // Get current positions from storage or create new
        let mut positions: Positions = env
            .storage()
            .persistent()
            .get(&MockPoolDataKey::Positions(owner.clone()))
            .unwrap_or_else(|| {
                let supply_map: Map<u32, i128> = Map::new(&env);
                let collateral_map: Map<u32, i128> = Map::new(&env);
                let liabilities_map: Map<u32, i128> = Map::new(&env);
                Positions {
                    collateral: collateral_map,
                    liabilities: liabilities_map,
                    supply: supply_map,
                }
            });

        let pool_address = env.current_contract_address();

        for request in requests.iter() {
            let token_client = token::TokenClient::new(&env, &request.address);
            if request.request_type == REQUEST_TYPE_SUPPLY_COLLATERAL {
                token_client.transfer_from(&pool_address, &spender, &pool_address, &request.amount);
                let current = positions.collateral.get(0).unwrap_or(0);
                positions.collateral.set(0, current + request.amount);
            } else if request.request_type == REQUEST_TYPE_WITHDRAW_COLLATERAL {
                token_client.transfer(&pool_address, &to, &request.amount);
                let current = positions.collateral.get(0).unwrap_or(0);
                positions.collateral.set(0, current - request.amount);
            }
        }

        // Store updated positions
        env.storage()
            .persistent()
            .set(&MockPoolDataKey::Positions(owner.clone()), &positions);

        positions
    }

    pub fn get_positions(env: Env, address: Address) -> Positions {
        // Return the stored positions or empty positions
        env.storage()
            .persistent()
            .get(&MockPoolDataKey::Positions(address))
            .unwrap_or_else(|| {
                let supply_map: Map<u32, i128> = Map::new(&env);
                let collateral_map: Map<u32, i128> = Map::new(&env);
                let liabilities_map: Map<u32, i128> = Map::new(&env);
                Positions {
                    collateral: collateral_map,
                    liabilities: liabilities_map,
                    supply: supply_map,
                }
            })
    }

    pub fn claim(env: Env, _from: Address, _reserve_token_ids: Vec<u32>, to: Address) -> i128 {
        // Mock returns 1000 BLND tokens (with 7 decimals = 0.001 BLND)
        let amount = 1000_0000000;
        if let Some(reward_token) = env
            .storage()
            .persistent()
            .get(&MockPoolDataKey::RewardToken)
        {
            let token_client = token::TokenClient::new(&env, &reward_token);
            token_client.transfer(&env.current_contract_address(), &to, &amount);
        }
        amount
    }
}

// Mock Comet Pool Contract for DEX swaps
#[contract]
pub struct MockCometPool;

#[contractimpl]
impl MockCometPool {
    pub fn swap_exact_amount_in(
        env: Env,
        token_in: Address,
        token_amount_in: i128,
        token_out: Address,
        min_amount_out: i128,
        _max_price: i128,
        user: Address,
    ) -> (i128, i128) {
        // Simple 1:1 mock swap ratio for testing
        // In reality BLND:USDC would have a different ratio
        let amount_out = token_amount_in;
        if amount_out < min_amount_out {
            panic!("insufficient output amount");
        }

        let contract = env.current_contract_address();

        // Pull BLND (or token_in) from the caller using allowance
        let token_in_client = token::TokenClient::new(&env, &token_in);
        token_in_client.transfer_from(&contract, &user, &contract, &token_amount_in);

        // Send token_out (USDC) from the pool to the user
        let token_out_client = token::TokenClient::new(&env, &token_out);
        token_out_client.transfer(&contract, &user, &amount_out);

        let spot_price = 1_0000000; // Mock spot price

        (amount_out, spot_price)
    }
}

// Improved Mock Blend Pool that actually transfers tokens like the real pool
#[contract]
pub struct RealisticMockBlendPool;

#[contractimpl]
impl RealisticMockBlendPool {
    pub fn set_reward_token(env: Env, token: Address) {
        env.storage()
            .persistent()
            .set(&MockPoolDataKey::RewardToken, &token);
    }

    pub fn submit(
        env: Env,
        owner: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        Self::process_requests(env, owner, spender, to, requests)
    }

    pub fn submit_with_allowance(
        env: Env,
        owner: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        Self::process_requests(env, owner, spender, to, requests)
    }

    fn process_requests(
        env: Env,
        owner: Address,
        spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        // Get current positions from storage or create new
        let mut positions: Positions = env
            .storage()
            .persistent()
            .get(&MockPoolDataKey::Positions(owner.clone()))
            .unwrap_or_else(|| {
                let supply_map: Map<u32, i128> = Map::new(&env);
                let collateral_map: Map<u32, i128> = Map::new(&env);
                let liabilities_map: Map<u32, i128> = Map::new(&env);
                Positions {
                    collateral: collateral_map,
                    liabilities: liabilities_map,
                    supply: supply_map,
                }
            });

        let pool_address = env.current_contract_address();

        for request in requests.iter() {
            let token_client = token::TokenClient::new(&env, &request.address);

            if request.request_type == REQUEST_TYPE_SUPPLY_COLLATERAL {
                token_client.transfer_from(&pool_address, &spender, &pool_address, &request.amount);

                let current = positions.collateral.get(0).unwrap_or(0);
                positions.collateral.set(0, current + request.amount);
            } else if request.request_type == REQUEST_TYPE_WITHDRAW_COLLATERAL {
                token_client.transfer(&pool_address, &to, &request.amount);

                let current = positions.collateral.get(0).unwrap_or(0);
                positions.collateral.set(0, current - request.amount);
            }
        }

        // Store updated positions
        env.storage()
            .persistent()
            .set(&MockPoolDataKey::Positions(owner.clone()), &positions);

        positions
    }

    pub fn get_positions(env: Env, address: Address) -> Positions {
        // Return the stored positions or empty positions
        env.storage()
            .persistent()
            .get(&MockPoolDataKey::Positions(address))
            .unwrap_or_else(|| {
                let supply_map: Map<u32, i128> = Map::new(&env);
                let collateral_map: Map<u32, i128> = Map::new(&env);
                let liabilities_map: Map<u32, i128> = Map::new(&env);
                Positions {
                    collateral: collateral_map,
                    liabilities: liabilities_map,
                    supply: supply_map,
                }
            })
    }

    pub fn claim(env: Env, _from: Address, _reserve_token_ids: Vec<u32>, to: Address) -> i128 {
        // Mock returns 1000 BLND tokens (with 7 decimals = 0.001 BLND)
        let amount = 1000_0000000;
        if let Some(reward_token) = env
            .storage()
            .persistent()
            .get(&MockPoolDataKey::RewardToken)
        {
            let token_client = token::TokenClient::new(&env, &reward_token);
            token_client.transfer(&env.current_contract_address(), &to, &amount);
        }
        amount
    }

    pub fn set_b_rate(env: Env, asset: Address, b_rate: i128) {
        store_b_rate(&env, &asset, b_rate);
    }

    pub fn get_reserve(env: Env, asset: Address) -> Reserve {
        build_reserve(asset.clone(), read_b_rate(&env, &asset))
    }
}
