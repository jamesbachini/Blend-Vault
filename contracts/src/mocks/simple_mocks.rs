#![cfg(test)]

use crate::{Positions, Request, REQUEST_TYPE_SUPPLY_COLLATERAL, REQUEST_TYPE_WITHDRAW_COLLATERAL};
use sep_41_token::testutils::MockTokenClient;
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Map, Vec};

// Storage keys for MockBlendPool
#[contracttype]
#[derive(Clone)]
pub enum MockPoolDataKey {
    Positions(Address),
}

// Mock Blend Pool Contract
#[contract]
pub struct MockBlendPool;

#[contractimpl]
impl MockBlendPool {
    pub fn submit(
        env: Env,
        _from: Address,
        _spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        Self::process_requests(env, to, requests)
    }

    pub fn submit_with_allowance(
        env: Env,
        _from: Address,
        _spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        // Same implementation as submit - in the real contract this would handle allowances
        Self::process_requests(env, to, requests)
    }

    fn process_requests(env: Env, to: Address, requests: Vec<Request>) -> Positions {
        // Get current positions from storage or create new
        let mut positions: Positions = env
            .storage()
            .persistent()
            .get(&MockPoolDataKey::Positions(to.clone()))
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

        // For each request, update the positions
        for request in requests.iter() {
            if request.request_type == REQUEST_TYPE_SUPPLY_COLLATERAL {
                // Get current collateral and add to it
                let current = positions.collateral.get(0).unwrap_or(0);
                positions.collateral.set(0, current + request.amount);
            } else if request.request_type == REQUEST_TYPE_WITHDRAW_COLLATERAL {
                // Get current collateral and subtract from it
                let current = positions.collateral.get(0).unwrap_or(0);
                positions.collateral.set(0, current - request.amount);
            }
        }

        // Store updated positions
        env.storage()
            .persistent()
            .set(&MockPoolDataKey::Positions(to.clone()), &positions);

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

    pub fn claim(
        _env: Env,
        _from: Address,
        _reserve_token_ids: Vec<u32>,
        _to: Address,
    ) -> i128 {
        // Mock returns 1000 BLND tokens (with 7 decimals = 0.001 BLND)
        1000_0000000
    }
}

// Mock Comet Pool Contract for DEX swaps
#[contract]
pub struct MockCometPool;

#[contractimpl]
impl MockCometPool {
    pub fn swap_exact_amount_in(
        _env: Env,
        _token_in: Address,
        token_amount_in: i128,
        _token_out: Address,
        _min_amount_out: i128,
        _max_price: i128,
        _user: Address,
    ) -> (i128, i128) {
        // Simple 1:1 mock swap ratio for testing
        // In reality BLND:USDC would have a different ratio
        let amount_out = token_amount_in;
        let spot_price = 1_0000000; // Mock spot price

        // For simplicity in tests, we don't actually transfer tokens
        // The mock just returns the swap amounts

        (amount_out, spot_price)
    }
}

// Improved Mock Blend Pool that actually transfers tokens like the real pool
#[contract]
pub struct RealisticMockBlendPool;

#[contractimpl]
impl RealisticMockBlendPool {
    pub fn submit(
        env: Env,
        _from: Address,
        _spender: Address,
        to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        Self::process_requests(env, to, requests)
    }

    fn process_requests(env: Env, to: Address, requests: Vec<Request>) -> Positions {
        // Get current positions from storage or create new
        let mut positions: Positions = env
            .storage()
            .persistent()
            .get(&MockPoolDataKey::Positions(to.clone()))
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

        // For each request, update the positions AND handle token transfers
        for request in requests.iter() {
            let token_client = token::TokenClient::new(&env, &request.address);

            if request.request_type == REQUEST_TYPE_SUPPLY_COLLATERAL {
                // For SUPPLY_COLLATERAL: Transfer tokens from vault to pool
                // The vault has pre-authorized this transfer via authorize_as_current_contract
                token_client.transfer(&to, &pool_address, &request.amount);

                // Update collateral position
                let current = positions.collateral.get(0).unwrap_or(0);
                positions.collateral.set(0, current + request.amount);
            } else if request.request_type == REQUEST_TYPE_WITHDRAW_COLLATERAL {
                // For WITHDRAW_COLLATERAL: Mint tokens to vault to simulate the pool returning funds
                // We use mint instead of transfer to avoid MockToken authorization issues
                let mock_token_client = MockTokenClient::new(&env, &request.address);
                mock_token_client.mint(&to, &request.amount);

                // Update collateral position
                let current = positions.collateral.get(0).unwrap_or(0);
                positions.collateral.set(0, current - request.amount);
            }
        }

        // Store updated positions
        env.storage()
            .persistent()
            .set(&MockPoolDataKey::Positions(to.clone()), &positions);

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

    pub fn claim(
        _env: Env,
        _from: Address,
        _reserve_token_ids: Vec<u32>,
        _to: Address,
    ) -> i128 {
        // Mock returns 1000 BLND tokens (with 7 decimals = 0.001 BLND)
        1000_0000000
    }
}
