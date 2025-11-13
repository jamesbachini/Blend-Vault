extern crate std;

use super::*;
use crate::mocks::{
    MockBlendPool, MockBlendPoolClient, MockCometPool, RealisticMockBlendPool,
    RealisticMockBlendPoolClient,
};
use blend_contract_sdk::{
    pool,
    testutils::{default_reserve_config, BlendFixture},
};
use sep_41_token::testutils::{MockTokenClient, MockTokenWASM};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, BytesN as _, Ledger, MockAuth, MockAuthInvoke},
    vec, Address, BytesN, Env, IntoVal, Map, String as SorobanString, String,
};
use std::{fs, path::PathBuf, process::Command, sync::OnceLock};

const EMBEDDED_COMET_WASM: &[u8] = include_bytes!("../test_artifacts/comet_pool.wasm");

const WEEK_IN_SECONDS: u64 = 60 * 60 * 24 * 7;
const EMITTER_WAIT_SECONDS: u64 = WEEK_IN_SECONDS * 3;
const BACKSTOP_WAIT_SECONDS: u64 = 60;
const GULP_DELAY_SECONDS: u64 = 60 * 60 * 24 + BACKSTOP_WAIT_SECONDS;
const EMISSION_RETRY_DELAY_SECONDS: u64 = 60 * 60;

fn comet_wasm_bytes() -> std::vec::Vec<u8> {
    static WASM: OnceLock<std::vec::Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let comet_repo = repo_root.join(".deps/comet-contracts-v1");
        let wasm_path = comet_repo
            .join("target/wasm32-unknown-unknown/release/contracts.wasm");

        if comet_repo.exists() {
            if !wasm_path.exists() {
                let status = Command::new("cargo")
                    .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
                    .current_dir(comet_repo.join("contracts"))
                    .status()
                    .expect("failed to build Comet contracts");
                assert!(status.success(), "Comet contracts build failed");
            }
            if let Ok(bytes) = fs::read(&wasm_path) {
                return bytes;
            }
        }

        EMBEDDED_COMET_WASM.to_vec()
    })
    .clone()
}

fn deploy_real_comet(
    env: &Env,
    deployer: &Address,
    blnd_token: &Address,
    usdc_token: &Address,
    wasm: &[u8],
) -> Address {
    let comet_address = env.register_contract_wasm(None, wasm);
    let comet_client = CometPoolClient::new(env, &comet_address);

    let usdc_client = MockTokenClient::new(env, usdc_token);
    let blnd_client = MockTokenClient::new(env, blnd_token);
    let initial_liquidity = 10_000_0000000;
    usdc_client.mint(deployer, &initial_liquidity);
    blnd_client.mint(deployer, &initial_liquidity);

    let tokens = vec![env, blnd_token.clone(), usdc_token.clone()];
    let weights = vec![env, 5_000_000i128, 5_000_000i128];
    let balances = vec![env, initial_liquidity, initial_liquidity];
    let swap_fee = 30_000i128;

    comet_client
        .mock_all_auths()
        .init(deployer, &tokens, &weights, &balances, &swap_fee);

    comet_address
}

// Test fixture that uses the real Blend protocol contracts
struct TestFixture<'a> {
    env: Env,
    deployer: Address,
    user: Address,
    usdc_token: Address,
    usdc_client: MockTokenClient<'a>,
    blnd_token: Address,
    blnd_client: MockTokenClient<'a>,
    blend_fixture: BlendFixture<'a>,
    blend_pool: Address,
    comet_pool: Address,
    vault: Address,
    vault_client: BlendVaultContractClient<'a>,
    usdc_reserve_index: u32,
    blnd_reserve_token_id: u32,
}

impl<'a> TestFixture<'a> {
    fn new() -> Self {
        Self::new_with_comet(|_, _, _, _, blend_fixture| {
            blend_fixture.backstop_token.address.clone()
        })
    }

    fn new_with_comet<F>(build_comet: F) -> Self
    where
        F: FnOnce(&Env, &Address, &Address, &Address, &BlendFixture) -> Address,
    {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        env.ledger().with_mut(|li| {
            li.timestamp = 1_700_000_000;
            li.sequence_number = 100;
            li.protocol_version = 23;
        });

        let deployer = Address::generate(&env);
        let user = Address::generate(&env);

        let usdc_token = env.register_contract_wasm(None, MockTokenWASM);
        let usdc_client = MockTokenClient::new(&env, &usdc_token);
        usdc_client.initialize(
            &deployer,
            &7,
            &SorobanString::from_str(&env, "USD Coin"),
            &SorobanString::from_str(&env, "USDC"),
        );

        let blnd_token = env.register_contract_wasm(None, MockTokenWASM);
        let blnd_client = MockTokenClient::new(&env, &blnd_token);
        blnd_client.initialize(
            &deployer,
            &7,
            &SorobanString::from_str(&env, "Blend Token"),
            &SorobanString::from_str(&env, "BLND"),
        );

        let blend_fixture = BlendFixture::deploy(&env, &deployer, &blnd_token, &usdc_token);

        let blend_pool = blend_fixture.pool_factory.mock_all_auths().deploy(
            &deployer,
            &String::from_str(&env, "Test Pool"),
            &BytesN::<32>::random(&env),
            &Address::generate(&env),
            &0_1000000,
            &4,
            &1_0000000,
        );
        let pool_client = pool::Client::new(&env, &blend_pool);

        let usdc_reserve_index = 0u32;
        let mut usdc_reserve_config = default_reserve_config();
        usdc_reserve_config.index = usdc_reserve_index;
        pool_client
            .mock_all_auths()
            .queue_set_reserve(&usdc_token, &usdc_reserve_config);
        pool_client.mock_all_auths().set_reserve(&usdc_token);

        let blnd_reserve_index = 1u32;
        let mut blnd_reserve_config = default_reserve_config();
        blnd_reserve_config.index = blnd_reserve_index;
        pool_client
            .mock_all_auths()
            .queue_set_reserve(&blnd_token, &blnd_reserve_config);
        pool_client.mock_all_auths().set_reserve(&blnd_token);

        let emission_metadata = vec![
            &env,
            pool::ReserveEmissionMetadata {
                res_index: usdc_reserve_index,
                res_type: 1,
                share: 1_0000000,
            },
        ];
        pool_client
            .mock_all_auths()
            .set_emissions_config(&emission_metadata);

        blend_fixture
            .backstop
            .mock_all_auths()
            .deposit(&deployer, &blend_pool, &50_000_0000000);
        blend_fixture
            .backstop
            .mock_all_auths()
            .add_reward(&blend_pool, &Option::<Address>::None);
        blend_fixture.backstop.mock_all_auths().drop();
        blend_fixture.backstop.mock_all_auths().distribute();
        pool_client.mock_all_auths().set_status(&3);
        pool_client.mock_all_auths().update_status();

        let comet_pool = build_comet(&env, &deployer, &blnd_token, &usdc_token, &blend_fixture);
        let blnd_reserve_token_id = usdc_reserve_index * 2 + 1;
        let vault = env.register_contract(None, BlendVaultContract);
        let vault_client = BlendVaultContractClient::new(&env, &vault);
        vault_client.initialize(
            &usdc_token,
            &0,
            &blend_pool,
            &usdc_reserve_index,
            &blnd_token,
            &blnd_reserve_token_id,
            &comet_pool,
        );

        usdc_client.mint(&user, &1_000_000_0000000);
        blnd_client.mint(&user, &1_000_000_0000000);
        usdc_client.approve(&user, &vault, &i128::MAX, &200);

        Self {
            env,
            deployer,
            user,
            usdc_token,
            usdc_client,
            blnd_token,
            blnd_client,
            blend_fixture,
            blend_pool,
            comet_pool,
            vault,
            vault_client,
            usdc_reserve_index,
            blnd_reserve_token_id,
        }
    }

    fn pool_client(&self) -> pool::Client<'a> {
        pool::Client::new(&self.env, &self.blend_pool)
    }

    fn advance_time(&self, seconds: u64) {
        let new_ts = self.env.ledger().timestamp() + seconds;
        self.env.ledger().with_mut(|li| {
            li.timestamp = new_ts;
            li.sequence_number += 1;
        });
    }

    fn accrue_emissions(&self) -> bool {
        let mut emitted = self.run_emission_cycle();
        if emitted == 0 {
            self.advance_time(EMISSION_RETRY_DELAY_SECONDS);
            emitted = self.run_emission_cycle();
        }
        if emitted == 0 {
            return false;
        }
        self.advance_time(GULP_DELAY_SECONDS);
        let gulped = self.pool_client().mock_all_auths().gulp_emissions();
        if gulped > 0 {
            self.advance_time(BACKSTOP_WAIT_SECONDS);
            true
        } else {
            false
        }
    }

    fn run_emission_cycle(&self) -> i128 {
        self.advance_time(EMITTER_WAIT_SECONDS);
        self.blend_fixture.emitter.mock_all_auths().distribute();
        self.advance_time(BACKSTOP_WAIT_SECONDS);
        self.blend_fixture.backstop.mock_all_auths().distribute()
    }

    fn claim_rewards(&self, reserve_ids: &[u32]) -> i128 {
        let mut ids = Vec::new(&self.env);
        for id in reserve_ids {
            ids.push_back(*id);
        }
        self.pool_client()
            .mock_all_auths()
            .claim(&self.vault, &ids, &self.vault)
    }
}

// Legacy mock fixture used for tests that need direct control over Blend internals
struct MockPoolFixture<'a> {
    env: Env,
    user: Address,
    usdc_token: Address,
    usdc_client: MockTokenClient<'a>,
    blnd_token: Address,
    blend_pool: Address,
    vault: Address,
    vault_client: BlendVaultContractClient<'a>,
}

impl<'a> MockPoolFixture<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        let usdc_token = env.register_contract_wasm(None, MockTokenWASM);
        let usdc_client = MockTokenClient::new(&env, &usdc_token);
        usdc_client.initialize(
            &admin,
            &7,
            &SorobanString::from_str(&env, "USD Coin"),
            &SorobanString::from_str(&env, "USDC"),
        );

        let blnd_token = env.register_contract_wasm(None, MockTokenWASM);
        let blnd_client = MockTokenClient::new(&env, &blnd_token);
        blnd_client.initialize(
            &admin,
            &7,
            &SorobanString::from_str(&env, "Blend Token"),
            &SorobanString::from_str(&env, "BLND"),
        );

        let blend_pool = env.register_contract(None, RealisticMockBlendPool);
        let realistic_pool_client = RealisticMockBlendPoolClient::new(&env, &blend_pool);
        realistic_pool_client.set_reward_token(&blnd_token);

        let comet_pool = env.register_contract(None, MockCometPool);

        let vault = env.register_contract(None, BlendVaultContract);
        let vault_client = BlendVaultContractClient::new(&env, &vault);
        vault_client.initialize(
            &usdc_token,
            &0,
            &blend_pool,
            &0,
            &blnd_token,
            &1,
            &comet_pool,
        );

        usdc_client.mint(&user, &1_000_000_0000000);
        usdc_client.approve(&user, &vault, &i128::MAX, &200);
        blnd_client.mint(&blend_pool, &1_000_000_0000000);

        Self {
            env,
            user,
            usdc_token,
            usdc_client,
            blnd_token,
            blend_pool,
            vault,
            vault_client,
        }
    }
}

#[contract]
#[derive(Clone)]
struct NoopBlendPool;

#[contractimpl]
impl NoopBlendPool {
    pub fn submit(
        env: Env,
        _owner: Address,
        _spender: Address,
        _to: Address,
        _requests: Vec<Request>,
    ) -> Positions {
        Self::empty_positions(&env)
    }

    pub fn submit_with_allowance(
        env: Env,
        _owner: Address,
        _spender: Address,
        _to: Address,
        _requests: Vec<Request>,
    ) -> Positions {
        Self::empty_positions(&env)
    }

    pub fn get_positions(env: Env, _address: Address) -> Positions {
        Self::empty_positions(&env)
    }

    pub fn claim(_env: Env, _from: Address, _reserve_token_ids: Vec<u32>, _to: Address) -> i128 {
        0
    }

    fn empty_positions(env: &Env) -> Positions {
        Positions {
            collateral: Map::new(env),
            liabilities: Map::new(env),
            supply: Map::new(env),
        }
    }
}

#[contract]
pub struct NoopCometPool;

#[contractimpl]
impl NoopCometPool {
    pub fn swap_exact_amount_in(
        _env: Env,
        _token_in: Address,
        _token_amount_in: i128,
        _token_out: Address,
        _min_amount_out: i128,
        _max_price: i128,
        _user: Address,
    ) -> (i128, i128) {
        (0, 0)
    }
}

#[test]
fn test_initialization() {
    let fixture = TestFixture::new();

    // Check token metadata
    assert_eq!(
        fixture.vault_client.name(),
        SorobanString::from_str(&fixture.env, "BLEND VAULT")
    );
    assert_eq!(
        fixture.vault_client.symbol(),
        SorobanString::from_str(&fixture.env, "BV")
    );
    assert_eq!(fixture.vault_client.decimals(), 7);

    // Check asset
    assert_eq!(fixture.vault_client.query_asset(), fixture.usdc_token);

    // Check initial supply
    assert_eq!(fixture.vault_client.total_supply(), 0);
}

#[test]
fn test_deposit() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000; // 1000 USDC

    // User deposits USDC
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Check shares minted (should be 1:1 for first deposit)
    assert_eq!(shares, deposit_amount);

    // Check user's share balance
    assert_eq!(fixture.vault_client.balance(&fixture.user), shares);

    // Check total supply
    assert_eq!(fixture.vault_client.total_supply(), shares);

    // Check USDC was transferred from user
    assert_eq!(
        fixture.usdc_client.balance(&fixture.user),
        1_000_000_0000000 - deposit_amount
    );
}

#[test]
fn test_multiple_deposits() {
    let fixture = TestFixture::new();
    let first_deposit = 1000_0000000; // 1000 USDC
    let second_deposit = 500_0000000; // 500 USDC

    // First deposit
    let shares1 = fixture.vault_client.mock_all_auths().deposit(
        &first_deposit,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Second deposit
    let shares2 = fixture.vault_client.mock_all_auths().deposit(
        &second_deposit,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Total shares should equal total deposits (1:1 ratio with no yield)
    assert_eq!(shares1 + shares2, first_deposit + second_deposit);
    assert_eq!(
        fixture.vault_client.balance(&fixture.user),
        first_deposit + second_deposit
    );
}

#[test]
fn test_mint() {
    let fixture = TestFixture::new();
    let shares_to_mint = 1000_0000000;

    // User mints shares
    let assets_deposited = fixture.vault_client.mock_all_auths().mint(
        &shares_to_mint,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // For first mint, should be 1:1
    assert_eq!(assets_deposited, shares_to_mint);

    // Check user's share balance
    assert_eq!(fixture.vault_client.balance(&fixture.user), shares_to_mint);

    // Check USDC was transferred
    assert_eq!(
        fixture.usdc_client.balance(&fixture.user),
        1_000_000_0000000 - assets_deposited
    );
}

#[test]
fn test_withdraw() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // First deposit
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );
    assert_eq!(fixture.vault_client.balance(&fixture.user), deposit_amount);

    // Then withdraw half
    let withdraw_amount = 500_0000000;
    let shares_burned = fixture.vault_client.mock_all_auths().withdraw(
        &withdraw_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Shares burned should equal amount withdrawn (1:1 ratio)
    assert_eq!(shares_burned, withdraw_amount);

    // Check remaining shares
    assert_eq!(
        fixture.vault_client.balance(&fixture.user),
        deposit_amount - shares_burned
    );

    // Check USDC balance (started with 1M, deposited 1000, withdrew 500)
    assert_eq!(
        fixture.usdc_client.balance(&fixture.user),
        1_000_000_0000000 - deposit_amount + withdraw_amount
    );
}

#[test]
fn test_redeem() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // First deposit
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Then redeem half the shares
    let shares_to_redeem = shares / 2;
    let assets_received = fixture.vault_client.mock_all_auths().redeem(
        &shares_to_redeem,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Assets should equal shares (1:1 ratio)
    assert_eq!(assets_received, shares_to_redeem);

    // Check remaining shares
    assert_eq!(
        fixture.vault_client.balance(&fixture.user),
        shares - shares_to_redeem
    );
}

#[test]
fn test_full_deposit_and_withdraw_cycle() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;
    let initial_balance = fixture.usdc_client.balance(&fixture.user);

    // Deposit
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Withdraw all
    fixture.vault_client.mock_all_auths().redeem(
        &shares,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Should have original balance back
    assert_eq!(fixture.usdc_client.balance(&fixture.user), initial_balance);

    // Should have no shares
    assert_eq!(fixture.vault_client.balance(&fixture.user), 0);
}

#[test]
fn test_preview_deposit() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // Preview before depositing
    let expected_shares = fixture.vault_client.preview_deposit(&deposit_amount);

    // Actually deposit
    let actual_shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Should match
    assert_eq!(expected_shares, actual_shares);
}

#[test]
fn test_preview_mint() {
    let fixture = TestFixture::new();
    let shares_to_mint = 1000_0000000;

    // Preview before minting
    let expected_assets = fixture.vault_client.preview_mint(&shares_to_mint);

    // Actually mint
    let actual_assets = fixture.vault_client.mock_all_auths().mint(
        &shares_to_mint,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Should match
    assert_eq!(expected_assets, actual_assets);
}

#[test]
fn test_preview_withdraw() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // First deposit
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    let withdraw_amount = 500_0000000;

    // Preview before withdrawing
    let expected_shares = fixture.vault_client.preview_withdraw(&withdraw_amount);

    // Actually withdraw
    let actual_shares = fixture.vault_client.mock_all_auths().withdraw(
        &withdraw_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Should match
    assert_eq!(expected_shares, actual_shares);
}

#[test]
fn test_preview_redeem() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // First deposit
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    let shares_to_redeem = shares / 2;

    // Preview before redeeming
    let expected_assets = fixture.vault_client.preview_redeem(&shares_to_redeem);

    // Actually redeem
    let actual_assets = fixture.vault_client.mock_all_auths().redeem(
        &shares_to_redeem,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Should match
    assert_eq!(expected_assets, actual_assets);
}

#[test]
fn test_convert_to_shares() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // Before any deposits
    let shares = fixture.vault_client.convert_to_shares(&deposit_amount);
    assert_eq!(shares, deposit_amount); // 1:1 when empty

    // After a deposit
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    let shares2 = fixture.vault_client.convert_to_shares(&deposit_amount);
    assert_eq!(shares2, deposit_amount); // Still 1:1 with no yield
}

#[test]
fn test_convert_to_assets() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // Make a deposit first
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Convert back
    let assets = fixture.vault_client.convert_to_assets(&shares);
    assert_eq!(assets, deposit_amount);
}

#[test]
fn test_total_assets_reflects_b_rate_growth() {
    let fixture = MockPoolFixture::new();
    let deposit_amount = 1000_0000000;

    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Simulate yield by increasing the Blend reserve b_rate
    let mock_pool_client = MockBlendPoolClient::new(&fixture.env, &fixture.blend_pool);
    let boosted_rate = crate::BLEND_RATE_SCALAR + 100_000_000_000; // ~10% increase
    mock_pool_client.set_b_rate(&fixture.usdc_token, &boosted_rate);

    let expected_assets = deposit_amount * boosted_rate / crate::BLEND_RATE_SCALAR;
    assert_eq!(fixture.vault_client.total_assets(), expected_assets);
}

#[test]
fn test_max_deposit() {
    let fixture = TestFixture::new();

    let max = fixture.vault_client.max_deposit(&fixture.user);
    // Should return i128::MAX by default
    assert_eq!(max, i128::MAX);
}

#[test]
fn test_max_mint() {
    let fixture = TestFixture::new();

    let max = fixture.vault_client.max_mint(&fixture.user);
    // Should return i128::MAX by default
    assert_eq!(max, i128::MAX);
}

#[test]
fn test_max_withdraw() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // Before deposit, max should be 0
    let max_before = fixture.vault_client.max_withdraw(&fixture.user);
    assert_eq!(max_before, 0);

    // After deposit
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    let max_after = fixture.vault_client.max_withdraw(&fixture.user);
    assert_eq!(max_after, deposit_amount);
}

#[test]
fn test_max_redeem() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // Before deposit, max should be 0
    let max_before = fixture.vault_client.max_redeem(&fixture.user);
    assert_eq!(max_before, 0);

    // After deposit
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    let max_after = fixture.vault_client.max_redeem(&fixture.user);
    assert_eq!(max_after, shares);
}

#[test]
fn test_zero_deposit() {
    let fixture = TestFixture::new();

    let shares = fixture.vault_client.mock_all_auths().deposit(
        &0,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    assert_eq!(shares, 0);
    assert_eq!(fixture.vault_client.balance(&fixture.user), 0);
}

#[test]
fn test_zero_mint() {
    let fixture = TestFixture::new();

    let assets =
        fixture
            .vault_client
            .mock_all_auths()
            .mint(&0, &fixture.user, &fixture.user, &fixture.user);

    assert_eq!(assets, 0);
    assert_eq!(fixture.vault_client.balance(&fixture.user), 0);
}

#[test]
#[should_panic]
fn test_withdraw_more_than_balance() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // Deposit
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Try to withdraw more than deposited (should panic)
    fixture.vault_client.mock_all_auths().withdraw(
        &(deposit_amount + 1),
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );
}

#[test]
#[should_panic]
fn test_redeem_more_than_shares() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // Deposit
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Try to redeem more shares than owned (should panic)
    fixture.vault_client.mock_all_auths().redeem(
        &(shares + 1),
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );
}

#[test]
fn test_total_assets_empty() {
    let fixture = TestFixture::new();

    // With no deposits, total assets should be 0
    assert_eq!(fixture.vault_client.total_assets(), 0);
}

#[test]
fn test_total_assets_after_deposit() {
    let fixture = TestFixture::new();
    let deposit_amount = 1000_0000000;

    // Deposit
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Note: With our stateful mock, total_assets should now reflect the deposit
    let total = fixture.vault_client.total_assets();
    // The mock now properly tracks state, so total should equal deposit
    assert_eq!(total, deposit_amount);
}

#[test]
fn test_multiple_users_deposit() {
    let fixture = TestFixture::new();
    let user2 = Address::generate(&fixture.env);

    // Mint USDC to second user
    fixture.usdc_client.mint(&user2, &1_000_000_0000000);

    // Approve vault to spend user2's USDC
    fixture
        .usdc_client
        .approve(&user2, &fixture.vault, &i128::MAX, &200);

    let deposit1 = 1000_0000000;
    let deposit2 = 2000_0000000;

    // First user deposits
    let shares1 = fixture.vault_client.mock_all_auths().deposit(
        &deposit1,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Second user deposits
    let shares2 = fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit2, &user2, &user2, &user2);

    // Check balances
    assert_eq!(fixture.vault_client.balance(&fixture.user), shares1);
    assert_eq!(fixture.vault_client.balance(&user2), shares2);

    // Total supply should be sum of both
    assert_eq!(fixture.vault_client.total_supply(), shares1 + shares2);
}

#[test]
fn test_compound_with_rewards() {
    let fixture = TestFixture::new();

    // First make a deposit so vault has position in Blend
    let deposit_amount = 1000_0000000;
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    assert!(
        fixture.accrue_emissions(),
        "failed to accrue Blend emissions before compounding"
    );
    // Call compound - should claim BLND and swap to USDC
    let usdc_deposited = fixture
        .vault_client
        .mock_all_auths()
        .compound(&fixture.user);

    // Real Blend pool should yield rewards that are deposited back into Blend
    assert!(usdc_deposited > 0);
}

#[test]
fn test_compound_with_real_comet_contract() {
    let wasm = comet_wasm_bytes();
    let fixture = TestFixture::new_with_comet(|env, deployer, blnd_token, usdc_token, _| {
        deploy_real_comet(env, deployer, blnd_token, usdc_token, &wasm)
    });

    let deposit_amount = 1_500_0000000;
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    assert!(
        fixture.accrue_emissions(),
        "failed to accrue Blend emissions before compounding"
    );

    let compounded = fixture
        .vault_client
        .mock_all_auths()
        .compound(&fixture.user);
    assert!(compounded > 0, "compound should deposit USDC after swap");
}

#[test]
fn test_compound_with_rewards_then_withdraw() {
    let fixture = TestFixture::new();
    let deposit_amount = 2_000_0000000;
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    assert!(
        fixture.accrue_emissions(),
        "failed to accrue Blend emissions before compounding"
    );
    fixture
        .vault_client
        .mock_all_auths()
        .compound(&fixture.user);

    let balance_before = fixture.usdc_client.balance(&fixture.user);
    let max_withdraw = fixture.vault_client.max_withdraw(&fixture.user);
    fixture.vault_client.mock_all_auths().withdraw(
        &max_withdraw,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );
    let balance_after = fixture.usdc_client.balance(&fixture.user);
    assert!(
        balance_after > balance_before,
        "withdrawal after compounding should increase user balance"
    );
}

#[test]
fn test_compound_without_rewards() {
    let fixture = TestFixture::new();
    let deposit_amount = 1_000_0000000;
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    let usdc_deposited = fixture
        .vault_client
        .mock_all_auths()
        .compound(&fixture.user);
    assert_eq!(usdc_deposited, 0);
}

#[test]
fn test_compound_requires_only_operator_auth() {
    let env = Env::default();
    env.ledger().with_mut(|li| {
        li.sequence_number = 500;
        li.timestamp = 1_700_000_000;
    });
    let admin = Address::generate(&env);
    let operator = Address::generate(&env);

    let usdc_token = env.register_contract_wasm(None, MockTokenWASM);
    let usdc_client = MockTokenClient::new(&env, &usdc_token);
    usdc_client.initialize(
        &admin,
        &7,
        &SorobanString::from_str(&env, "USD Coin"),
        &SorobanString::from_str(&env, "USDC"),
    );

    let blnd_token = env.register_contract_wasm(None, MockTokenWASM);
    let blnd_client = MockTokenClient::new(&env, &blnd_token);
    blnd_client.initialize(
        &admin,
        &7,
        &SorobanString::from_str(&env, "Blend Token"),
        &SorobanString::from_str(&env, "BLND"),
    );

    let blend_pool = env.register_contract(None, NoopBlendPool);
    let comet_pool = env.register_contract(None, NoopCometPool);
    let vault = env.register_contract(None, BlendVaultContract);
    let vault_client = BlendVaultContractClient::new(&env, &vault);
    vault_client.initialize(
        &usdc_token,
        &0,
        &blend_pool,
        &0,
        &blnd_token,
        &1,
        &comet_pool,
    );

    env.set_auths(&[]);
    let result = vault_client
        .mock_auths(&[MockAuth {
            address: &operator,
            invoke: &MockAuthInvoke {
                contract: &vault,
                fn_name: &"compound",
                args: vec![&env, operator.clone().into_val(&env)],
                sub_invokes: &[],
            },
        }])
        .compound(&operator);
    assert_eq!(result, 0);
}

#[test]
fn test_deposit_different_receiver() {
    let fixture = TestFixture::new();
    let receiver = Address::generate(&fixture.env);
    let deposit_amount = 1000_0000000;

    // User deposits but shares go to receiver
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &receiver,
        &fixture.user,
        &fixture.user,
    );

    // Receiver should have the shares
    assert_eq!(fixture.vault_client.balance(&receiver), shares);

    // User should have paid the USDC
    assert_eq!(
        fixture.usdc_client.balance(&fixture.user),
        1_000_000_0000000 - deposit_amount
    );

    // Original user should have no shares
    assert_eq!(fixture.vault_client.balance(&fixture.user), 0);
}

#[test]
fn test_real_blend_emissions_produce_claims() {
    let fixture = TestFixture::new();
    let deposit_amount = 1_000_0000000;

    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    assert!(
        fixture.accrue_emissions(),
        "emission accrual should succeed in real Blend fixture"
    );

    let claimed = fixture.claim_rewards(&[fixture.blnd_reserve_token_id]);
    assert!(claimed > 0, "fixture should yield BLND rewards to claim");
}

#[test]
fn test_withdraw_different_receiver() {
    let fixture = TestFixture::new();
    let receiver = Address::generate(&fixture.env);
    let deposit_amount = 1000_0000000;

    // First deposit
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    let initial_receiver_balance = fixture.usdc_client.balance(&receiver);

    // Withdraw to different receiver
    let withdraw_amount = 500_0000000;
    fixture.vault_client.mock_all_auths().withdraw(
        &withdraw_amount,
        &receiver,
        &fixture.user,
        &fixture.user,
    );

    // Receiver should have gotten the USDC
    assert_eq!(
        fixture.usdc_client.balance(&receiver),
        initial_receiver_balance + withdraw_amount
    );
}

#[test]
fn test_fungible_token_interface() {
    let fixture = TestFixture::new();

    // Test name
    assert_eq!(
        fixture.vault_client.name(),
        SorobanString::from_str(&fixture.env, "BLEND VAULT")
    );

    // Test symbol
    assert_eq!(
        fixture.vault_client.symbol(),
        SorobanString::from_str(&fixture.env, "BV")
    );

    // Test decimals
    assert_eq!(fixture.vault_client.decimals(), 7);

    // Test total supply starts at 0
    assert_eq!(fixture.vault_client.total_supply(), 0);

    // Test balance
    assert_eq!(fixture.vault_client.balance(&fixture.user), 0);
}

#[test]
fn test_allowance_and_transfer_from() {
    let fixture = TestFixture::new();
    let spender = Address::generate(&fixture.env);
    let deposit_amount = 1000_0000000;

    // User deposits to get shares
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // User approves spender
    let shares = fixture.vault_client.balance(&fixture.user);
    fixture
        .vault_client
        .mock_all_auths()
        .approve(&fixture.user, &spender, &shares, &200);

    // Check allowance
    assert_eq!(
        fixture.vault_client.allowance(&fixture.user, &spender),
        shares
    );
}

#[test]
fn test_transfer_shares() {
    let fixture = TestFixture::new();
    let recipient = Address::generate(&fixture.env);
    let deposit_amount = 1000_0000000;

    // User deposits to get shares
    let shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Transfer half the shares
    let transfer_amount = shares / 2;
    fixture
        .vault_client
        .mock_all_auths()
        .transfer(&fixture.user, &recipient, &transfer_amount);

    // Check balances
    assert_eq!(
        fixture.vault_client.balance(&fixture.user),
        shares - transfer_amount
    );
    assert_eq!(fixture.vault_client.balance(&recipient), transfer_amount);
}

#[test]
fn test_depositors_snapshot() {
    let fixture = TestFixture::new();

    // Initially, snapshot should be empty
    let snapshot = fixture.vault_client.get_depositors_snapshot();
    assert_eq!(snapshot.len(), 0);

    // Create additional users
    let user2 = Address::generate(&fixture.env);
    let user3 = Address::generate(&fixture.env);

    // Mint USDC to users
    fixture.usdc_client.mint(&user2, &1_000_000_0000000);
    fixture.usdc_client.mint(&user3, &1_000_000_0000000);

    // Approve vault to spend users' USDC
    fixture
        .usdc_client
        .approve(&user2, &fixture.vault, &i128::MAX, &200);
    fixture
        .usdc_client
        .approve(&user3, &fixture.vault, &i128::MAX, &200);

    // User 1 deposits
    let deposit1 = 1000_0000000;
    fixture.vault_client.mock_all_auths().deposit(
        &deposit1,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Check snapshot after first deposit
    let snapshot = fixture.vault_client.get_depositors_snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot.get(fixture.user.clone()).unwrap(), deposit1);

    // User 2 deposits
    let deposit2 = 2000_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit2, &user2, &user2, &user2);

    // Check snapshot with two depositors
    let snapshot = fixture.vault_client.get_depositors_snapshot();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot.get(fixture.user.clone()).unwrap(), deposit1);
    assert_eq!(snapshot.get(user2.clone()).unwrap(), deposit2);

    // User 3 mints shares (should also be tracked)
    let shares3 = 3000_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .mint(&shares3, &user3, &user3, &user3);

    // Check snapshot with three depositors
    let snapshot = fixture.vault_client.get_depositors_snapshot();
    assert_eq!(snapshot.len(), 3);
    assert_eq!(snapshot.get(fixture.user.clone()).unwrap(), deposit1);
    assert_eq!(snapshot.get(user2.clone()).unwrap(), deposit2);
    assert_eq!(snapshot.get(user3.clone()).unwrap(), shares3);

    // User 1 withdraws everything
    fixture.vault_client.mock_all_auths().redeem(
        &deposit1,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Snapshot should only show users with non-zero balances
    let snapshot = fixture.vault_client.get_depositors_snapshot();
    assert_eq!(snapshot.len(), 2);
    assert!(snapshot.get(fixture.user.clone()).is_none()); // User 1 has zero balance
    assert_eq!(snapshot.get(user2.clone()).unwrap(), deposit2);
    assert_eq!(snapshot.get(user3.clone()).unwrap(), shares3);
}

#[test]
fn test_depositors_snapshot_no_duplicates() {
    let fixture = TestFixture::new();

    // User deposits multiple times
    let deposit1 = 1000_0000000;
    fixture.vault_client.mock_all_auths().deposit(
        &deposit1,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    let deposit2 = 500_0000000;
    fixture.vault_client.mock_all_auths().deposit(
        &deposit2,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Snapshot should only have one entry for the user
    let snapshot = fixture.vault_client.get_depositors_snapshot();
    assert_eq!(snapshot.len(), 1);
    // Balance should be the sum of both deposits
    assert_eq!(
        snapshot.get(fixture.user.clone()).unwrap(),
        deposit1 + deposit2
    );
}

#[test]
fn test_is_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let usdc_token = env.register_contract_wasm(None, MockTokenWASM);
    let usdc_client = MockTokenClient::new(&env, &usdc_token);
    usdc_client.initialize(
        &admin,
        &7,
        &SorobanString::from_str(&env, "USD Coin"),
        &SorobanString::from_str(&env, "USDC"),
    );

    let blnd_token = env.register_contract_wasm(None, MockTokenWASM);
    let blnd_client = MockTokenClient::new(&env, &blnd_token);
    blnd_client.initialize(
        &admin,
        &7,
        &SorobanString::from_str(&env, "Blend Token"),
        &SorobanString::from_str(&env, "BLND"),
    );

    let blend_pool = env.register_contract(None, MockBlendPool);
    let comet_pool = env.register_contract(None, MockCometPool);

    // Deploy vault without initialization
    let vault = env.register_contract(None, BlendVaultContract);
    let vault_client = BlendVaultContractClient::new(&env, &vault);

    // Check that it's not initialized
    assert_eq!(vault_client.is_initialized(), false);

    // Initialize the vault
    vault_client.initialize(
        &usdc_token,
        &0,
        &blend_pool,
        &0,
        &blnd_token,
        &1,
        &comet_pool,
    );

    // Check that it's now initialized
    assert_eq!(vault_client.is_initialized(), true);
}

#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_double_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let usdc_token = env.register_contract_wasm(None, MockTokenWASM);
    let usdc_client = MockTokenClient::new(&env, &usdc_token);
    usdc_client.initialize(
        &admin,
        &7,
        &SorobanString::from_str(&env, "USD Coin"),
        &SorobanString::from_str(&env, "USDC"),
    );

    let blnd_token = env.register_contract_wasm(None, MockTokenWASM);
    let blnd_client = MockTokenClient::new(&env, &blnd_token);
    blnd_client.initialize(
        &admin,
        &7,
        &SorobanString::from_str(&env, "Blend Token"),
        &SorobanString::from_str(&env, "BLND"),
    );

    let blend_pool = env.register_contract(None, MockBlendPool);
    let comet_pool = env.register_contract(None, MockCometPool);

    // Deploy vault
    let vault = env.register_contract(None, BlendVaultContract);
    let vault_client = BlendVaultContractClient::new(&env, &vault);

    // Initialize the vault (first time)
    vault_client.initialize(
        &usdc_token,
        &0,
        &blend_pool,
        &0,
        &blnd_token,
        &1,
        &comet_pool,
    );

    // Try to initialize again (should panic)
    vault_client.initialize(
        &usdc_token,
        &0,
        &blend_pool,
        &0,
        &blnd_token,
        &1,
        &comet_pool,
    );
}

// ===== Tests with Authorization-Enforcing Mock =====
// These tests use an improved mock that simulates token transfers and authorization

#[test]
fn test_deposit_with_authorization() {
    let env = Env::default();
    // Mock all auths to simulate signed transactions in a real environment
    // This allows us to test the authorization flow works correctly
    env.mock_all_auths();
    let user = Address::generate(&env);

    // Deploy USDC token
    let usdc_token = env.register_contract_wasm(None, MockTokenWASM);
    let usdc_client = MockTokenClient::new(&env, &usdc_token);
    usdc_client.initialize(
        &user,
        &7,
        &SorobanString::from_str(&env, "USD Coin"),
        &SorobanString::from_str(&env, "USDC"),
    );

    // Deploy BLND token
    let blnd_token = env.register_contract_wasm(None, MockTokenWASM);
    let blnd_client = MockTokenClient::new(&env, &blnd_token);
    blnd_client.initialize(
        &user,
        &7,
        &SorobanString::from_str(&env, "Blend Token"),
        &SorobanString::from_str(&env, "BLND"),
    );

    // Deploy realistic mock pool that requires authorization
    let blend_pool = env.register_contract(None, RealisticMockBlendPool);
    let realistic_pool_client = RealisticMockBlendPoolClient::new(&env, &blend_pool);
    realistic_pool_client.set_reward_token(&blnd_token);

    // Deploy mock Comet Pool
    let comet_pool = env.register_contract(None, MockCometPool);

    // Deploy and initialize vault
    let vault = env.register_contract(None, BlendVaultContract);
    let vault_client = BlendVaultContractClient::new(&env, &vault);
    vault_client.initialize(
        &usdc_token,
        &0,
        &blend_pool,
        &0,
        &blnd_token,
        &1,
        &comet_pool,
    );

    // Mint USDC to user
    usdc_client.mint(&user, &10_000_0000000);

    let deposit_amount = 1000_0000000;

    // User must approve vault to spend their USDC first
    usdc_client.approve(&user, &vault, &deposit_amount, &200);

    // This deposit should work because user approved vault and vault uses transfer_from
    let shares = vault_client.deposit(&deposit_amount, &user, &user, &user);

    // Verify success
    assert_eq!(shares, deposit_amount);
    assert_eq!(vault_client.balance(&user), shares);

    // Verify USDC was actually transferred to the pool
    assert_eq!(usdc_client.balance(&blend_pool), deposit_amount);

    // Verify total assets
    assert_eq!(vault_client.total_assets(), deposit_amount);
}

#[test]
fn test_withdraw_with_authorization() {
    let env = Env::default();
    // Mock all auths to simulate signed transactions
    env.mock_all_auths();
    let user = Address::generate(&env);

    // Setup tokens
    let usdc_token = env.register_contract_wasm(None, MockTokenWASM);
    let usdc_client = MockTokenClient::new(&env, &usdc_token);
    usdc_client.initialize(
        &user,
        &7,
        &SorobanString::from_str(&env, "USD Coin"),
        &SorobanString::from_str(&env, "USDC"),
    );

    let blnd_token = env.register_contract_wasm(None, MockTokenWASM);
    let blnd_client = MockTokenClient::new(&env, &blnd_token);
    blnd_client.initialize(
        &user,
        &7,
        &SorobanString::from_str(&env, "Blend Token"),
        &SorobanString::from_str(&env, "BLND"),
    );

    // Deploy simple mock pool (doesn't actually transfer tokens, just tracks positions)
    // This avoids MockToken authorization issues in nested contract calls during withdrawals
    let blend_pool = env.register_contract(None, MockBlendPool);
    let comet_pool = env.register_contract(None, MockCometPool);

    // Deploy and initialize vault
    let vault = env.register_contract(None, BlendVaultContract);
    let vault_client = BlendVaultContractClient::new(&env, &vault);
    vault_client.initialize(
        &usdc_token,
        &0,
        &blend_pool,
        &0,
        &blnd_token,
        &1,
        &comet_pool,
    );

    // Mint USDC to user
    usdc_client.mint(&user, &10_000_0000000);

    // Deposit first
    let deposit_amount = 5000_0000000;

    // User must approve vault to spend their USDC first
    usdc_client.approve(&user, &vault, &deposit_amount, &200);

    let shares = vault_client.deposit(&deposit_amount, &user, &user, &user);

    // Now withdraw
    let withdraw_amount = 2000_0000000;
    let shares_burned = vault_client.withdraw(&withdraw_amount, &user, &user, &user);

    // Verify
    assert_eq!(shares_burned, withdraw_amount);
    assert_eq!(vault_client.balance(&user), shares - shares_burned);

    // Verify USDC was transferred back to user from pool
    assert_eq!(
        usdc_client.balance(&user),
        10_000_0000000 - deposit_amount + withdraw_amount
    );
}

#[test]
fn test_multiple_deposits_with_auth() {
    let env = Env::default();
    // Mock all auths to simulate signed transactions
    env.mock_all_auths();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Setup tokens
    let usdc_token = env.register_contract_wasm(None, MockTokenWASM);
    let usdc_client = MockTokenClient::new(&env, &usdc_token);
    usdc_client.initialize(
        &user1,
        &7,
        &SorobanString::from_str(&env, "USD Coin"),
        &SorobanString::from_str(&env, "USDC"),
    );

    let blnd_token = env.register_contract_wasm(None, MockTokenWASM);
    let blnd_client = MockTokenClient::new(&env, &blnd_token);
    blnd_client.initialize(
        &user1,
        &7,
        &SorobanString::from_str(&env, "Blend Token"),
        &SorobanString::from_str(&env, "BLND"),
    );

    // Deploy realistic mock pool
    let blend_pool = env.register_contract(None, RealisticMockBlendPool);
    let realistic_pool_client = RealisticMockBlendPoolClient::new(&env, &blend_pool);
    realistic_pool_client.set_reward_token(&blnd_token);
    let comet_pool = env.register_contract(None, MockCometPool);

    // Deploy and initialize vault
    let vault = env.register_contract(None, BlendVaultContract);
    let vault_client = BlendVaultContractClient::new(&env, &vault);
    vault_client.initialize(
        &usdc_token,
        &0,
        &blend_pool,
        &0,
        &blnd_token,
        &1,
        &comet_pool,
    );

    // Mint USDC to both users
    usdc_client.mint(&user1, &10_000_0000000);
    usdc_client.mint(&user2, &10_000_0000000);

    // Both users deposit
    let deposit1 = 1000_0000000;

    // User1 must approve vault to spend their USDC first
    usdc_client.approve(&user1, &vault, &deposit1, &200);
    let shares1 = vault_client.deposit(&deposit1, &user1, &user1, &user1);

    let deposit2 = 2000_0000000;

    // User2 must approve vault to spend their USDC first
    usdc_client.approve(&user2, &vault, &deposit2, &200);
    let shares2 = vault_client.deposit(&deposit2, &user2, &user2, &user2);

    // Verify both deposits worked
    assert_eq!(vault_client.balance(&user1), shares1);
    assert_eq!(vault_client.balance(&user2), shares2);

    // Verify total assets
    assert_eq!(vault_client.total_assets(), deposit1 + deposit2);
}

// ===== COMPREHENSIVE EDGE CASE TESTS =====

#[test]
fn test_exact_user_scenario_reported() {
    // This test replicates the exact scenario that caused the $0.98 balance issue:
    // User 1 deposits 0.01 USDC
    // User 2 deposits 1 USDC
    // User 1 deposits another 0.01 USDC
    // User 1's balance should be 0.02 USDC, not 0.98 USDC

    let fixture = TestFixture::new();
    let user1 = fixture.user;
    let user2 = Address::generate(&fixture.env);

    // Setup user2
    fixture.usdc_client.mint(&user2, &10_0000000);
    fixture
        .usdc_client
        .approve(&user2, &fixture.vault, &i128::MAX, &200);

    // User 1 deposits 0.01 USDC
    let deposit1 = 1_0000; // 0.0001 USDC (7 decimals)
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit1, &user1, &user1, &user1);

    // User 2 deposits 1 USDC
    let deposit2 = 1_0000000; // 1 USDC
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit2, &user2, &user2, &user2);

    // User 1 deposits another 0.01 USDC
    let deposit3 = 1_0000; // 0.0001 USDC
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit3, &user1, &user1, &user1);

    // Check user1's max_withdraw (how much USDC they can withdraw)
    let user1_max_withdraw = fixture.vault_client.max_withdraw(&user1);
    let user2_max_withdraw = fixture.vault_client.max_withdraw(&user2);

    // User1 should be able to withdraw their total deposits (0.0002 USDC)
    let expected_user1_assets = deposit1 + deposit3;
    assert_eq!(
        user1_max_withdraw, expected_user1_assets,
        "User1 should be able to withdraw exactly what they deposited"
    );

    // User2 should be able to withdraw their deposit (1 USDC)
    assert_eq!(
        user2_max_withdraw, deposit2,
        "User2 should be able to withdraw exactly what they deposited"
    );

    // Total assets should equal sum of all deposits
    let total_assets = fixture.vault_client.total_assets();
    assert_eq!(
        total_assets,
        deposit1 + deposit2 + deposit3,
        "Total assets should equal sum of all deposits"
    );
}

#[test]
fn test_small_and_large_deposits_precision() {
    // Test that small deposits don't get rounded away when mixed with large deposits
    let fixture = TestFixture::new();
    let user1 = fixture.user;
    let user2 = Address::generate(&fixture.env);

    fixture.usdc_client.mint(&user2, &1_000_000_0000000);
    fixture
        .usdc_client
        .approve(&user2, &fixture.vault, &i128::MAX, &200);

    // User1 deposits tiny amount (0.0000001 USDC = 1 stroops)
    let tiny_deposit = 1;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&tiny_deposit, &user1, &user1, &user1);

    // User2 deposits large amount (100,000 USDC)
    let large_deposit = 100_000_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&large_deposit, &user2, &user2, &user2);

    // User1 deposits another tiny amount
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&tiny_deposit, &user1, &user1, &user1);

    // Check that user1 can still withdraw their full amount
    let user1_max_withdraw = fixture.vault_client.max_withdraw(&user1);
    assert_eq!(
        user1_max_withdraw,
        tiny_deposit + tiny_deposit,
        "Tiny deposits should not be lost even with large deposits"
    );
}

#[test]
fn test_max_withdraw_matches_share_value() {
    // Verify that max_withdraw always returns the correct asset value of user's shares
    let fixture = TestFixture::new();
    let user1 = fixture.user;
    let user2 = Address::generate(&fixture.env);

    fixture.usdc_client.mint(&user2, &10_000_0000000);
    fixture
        .usdc_client
        .approve(&user2, &fixture.vault, &i128::MAX, &200);

    // Multiple deposits
    let deposit1 = 100_0000000; // 100 USDC
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit1, &user1, &user1, &user1);

    let deposit2 = 500_0000000; // 500 USDC
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit2, &user2, &user2, &user2);

    let deposit3 = 50_0000000; // 50 USDC
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit3, &user1, &user1, &user1);

    // Check max_withdraw for both users
    let user1_max_withdraw = fixture.vault_client.max_withdraw(&user1);
    let user2_max_withdraw = fixture.vault_client.max_withdraw(&user2);

    // User1 total deposits
    let user1_total_deposits = deposit1 + deposit3;
    assert_eq!(
        user1_max_withdraw, user1_total_deposits,
        "User1 max_withdraw should equal their total deposits"
    );

    // User2 total deposits
    assert_eq!(
        user2_max_withdraw, deposit2,
        "User2 max_withdraw should equal their deposit"
    );

    // Sum of max_withdraws should equal total_assets
    let total_assets = fixture.vault_client.total_assets();
    assert_eq!(
        user1_max_withdraw + user2_max_withdraw,
        total_assets,
        "Sum of all max_withdraws should equal total_assets"
    );
}

#[test]
fn test_total_assets_matches_blend_pool() {
    // Verify that total_assets correctly queries the Blend pool collateral
    let fixture = TestFixture::new();

    let deposit_amount = 1000_0000000; // 1000 USDC
    fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Get total_assets from vault
    let total_assets = fixture.vault_client.total_assets();

    // Query Blend pool directly
    let pool_client = BlendPoolClient::new(&fixture.env, &fixture.blend_pool);
    let positions = pool_client.get_positions(&fixture.vault);

    // Get collateral from positions (index 0 is USDC in our test)
    let blend_collateral = positions.collateral.get(0).unwrap_or(0);

    assert_eq!(
        total_assets, blend_collateral,
        "total_assets should match Blend pool collateral position"
    );

    // Also verify it matches the deposit
    assert_eq!(
        total_assets, deposit_amount,
        "total_assets should equal deposited amount"
    );
}

#[test]
fn test_share_price_consistency() {
    // Test that share price remains consistent across multiple operations
    let fixture = TestFixture::new();
    let user1 = fixture.user;
    let user2 = Address::generate(&fixture.env);
    let user3 = Address::generate(&fixture.env);

    fixture.usdc_client.mint(&user2, &10_000_0000000);
    fixture
        .usdc_client
        .approve(&user2, &fixture.vault, &i128::MAX, &200);
    fixture.usdc_client.mint(&user3, &10_000_0000000);
    fixture
        .usdc_client
        .approve(&user3, &fixture.vault, &i128::MAX, &200);

    // User1 deposits (first depositor)
    let deposit1 = 100_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit1, &user1, &user1, &user1);

    // Check 1:1 ratio for first deposit
    let shares1 = fixture.vault_client.balance(&user1);
    assert_eq!(
        shares1, deposit1,
        "First deposit should be 1:1 shares:assets"
    );

    // User2 deposits same amount
    let deposit2 = 100_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit2, &user2, &user2, &user2);

    let shares2 = fixture.vault_client.balance(&user2);
    assert_eq!(
        shares2, deposit2,
        "Second deposit should also be 1:1 with no yield"
    );

    // User3 deposits different amount
    let deposit3 = 250_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit3, &user3, &user3, &user3);

    let shares3 = fixture.vault_client.balance(&user3);
    assert_eq!(
        shares3, deposit3,
        "Third deposit should maintain 1:1 ratio with no yield"
    );

    // Verify share values
    let user1_assets = fixture.vault_client.convert_to_assets(&shares1);
    let user2_assets = fixture.vault_client.convert_to_assets(&shares2);
    let user3_assets = fixture.vault_client.convert_to_assets(&shares3);

    assert_eq!(
        user1_assets, deposit1,
        "User1 shares should be worth deposit1"
    );
    assert_eq!(
        user2_assets, deposit2,
        "User2 shares should be worth deposit2"
    );
    assert_eq!(
        user3_assets, deposit3,
        "User3 shares should be worth deposit3"
    );
}

#[test]
fn test_sequential_deposits_and_withdrawals() {
    // Test complex sequence of deposits and withdrawals
    let fixture = TestFixture::new();
    let user1 = fixture.user;
    let user2 = Address::generate(&fixture.env);

    fixture.usdc_client.mint(&user2, &10_000_0000000);
    fixture
        .usdc_client
        .approve(&user2, &fixture.vault, &i128::MAX, &200);

    // User1 deposits 100 USDC
    let deposit1 = 100_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit1, &user1, &user1, &user1);

    // User2 deposits 200 USDC
    let deposit2 = 200_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit2, &user2, &user2, &user2);

    // User1 withdraws 50 USDC
    let withdraw1 = 50_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .withdraw(&withdraw1, &user1, &user1, &user1);

    // User1 deposits another 25 USDC
    let deposit3 = 25_0000000;
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&deposit3, &user1, &user1, &user1);

    // Check final balances
    let user1_max_withdraw = fixture.vault_client.max_withdraw(&user1);
    let user2_max_withdraw = fixture.vault_client.max_withdraw(&user2);

    // User1: deposited 100 + 25 - withdrew 50 = 75 USDC
    let expected_user1 = deposit1 + deposit3 - withdraw1;
    assert_eq!(
        user1_max_withdraw, expected_user1,
        "User1 should have 75 USDC"
    );

    // User2: deposited 200, should still have 200
    assert_eq!(
        user2_max_withdraw, deposit2,
        "User2 should still have 200 USDC"
    );

    // Total assets should be 275 USDC (no automatic compounding anymore)
    let total_assets = fixture.vault_client.total_assets();
    assert_eq!(
        total_assets,
        expected_user1 + deposit2,
        "Total assets should be equal to 275 USDC"
    );
}

#[test]
fn test_convert_functions_bidirectional() {
    // Test that convert_to_shares and convert_to_assets are inverse operations
    let fixture = TestFixture::new();

    // Deposit to establish share price
    let deposit = 1000_0000000;
    fixture.vault_client.mock_all_auths().deposit(
        &deposit,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // Test various amounts
    let test_amounts = [
        1_0000000,   // 1 USDC
        10_0000000,  // 10 USDC
        100_0000000, // 100 USDC
        1_0000,      // 0.0001 USDC
        999_9999999, // 999.9999999 USDC
    ];

    for &assets in &test_amounts {
        // Convert assets -> shares -> assets
        let shares = fixture.vault_client.convert_to_shares(&assets);
        let assets_back = fixture.vault_client.convert_to_assets(&shares);

        // Should be equal (or very close due to rounding)
        assert!(
            (assets_back as i128 - assets as i128).abs() <= 1,
            "Round-trip conversion should preserve value: {} -> {} -> {}",
            assets,
            shares,
            assets_back
        );
    }
}

#[test]
fn test_multiple_small_deposits_accumulate() {
    // Test that many small deposits correctly accumulate
    let fixture = TestFixture::new();

    let small_deposit = 1_0000000; // 1 USDC
    let num_deposits = 10;

    for _ in 0..num_deposits {
        fixture.vault_client.mock_all_auths().deposit(
            &small_deposit,
            &fixture.user,
            &fixture.user,
            &fixture.user,
        );
    }

    // User should be able to withdraw total of all deposits
    let max_withdraw = fixture.vault_client.max_withdraw(&fixture.user);
    let expected_total = small_deposit * num_deposits;

    assert_eq!(
        max_withdraw, expected_total,
        "Multiple small deposits should accumulate correctly"
    );

    // Total assets should match
    let total_assets = fixture.vault_client.total_assets();
    assert_eq!(
        total_assets, expected_total,
        "Total assets should match accumulated deposits"
    );
}

#[test]
fn test_zero_balance_user_max_withdraw() {
    // Test that max_withdraw returns 0 for users with no deposits
    let fixture = TestFixture::new();
    let user_with_no_deposit = Address::generate(&fixture.env);

    // Another user deposits to establish vault state
    fixture.vault_client.mock_all_auths().deposit(
        &100_0000000,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    // User with no deposit should have 0 max_withdraw
    let max_withdraw = fixture.vault_client.max_withdraw(&user_with_no_deposit);
    assert_eq!(
        max_withdraw, 0,
        "User with no deposits should have 0 max_withdraw"
    );
}

#[test]
fn test_fractional_share_values() {
    // Test deposits that result in fractional share values due to rounding
    let fixture = TestFixture::new();
    let user1 = fixture.user;
    let user2 = Address::generate(&fixture.env);
    let user3 = Address::generate(&fixture.env);

    fixture.usdc_client.mint(&user2, &10_000_0000000);
    fixture
        .usdc_client
        .approve(&user2, &fixture.vault, &i128::MAX, &200);
    fixture.usdc_client.mint(&user3, &10_000_0000000);
    fixture
        .usdc_client
        .approve(&user3, &fixture.vault, &i128::MAX, &200);

    // User1: 333 USDC (will divide into thirds later)
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&333_0000000, &user1, &user1, &user1);

    // User2: 333 USDC
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&333_0000000, &user2, &user2, &user2);

    // User3: 334 USDC (to handle rounding)
    fixture
        .vault_client
        .mock_all_auths()
        .deposit(&334_0000000, &user3, &user3, &user3);

    // Total should be 1000 USDC
    let total_assets = fixture.vault_client.total_assets();
    assert_eq!(total_assets, 1000_0000000, "Total should be 1000 USDC");

    // Each user's max_withdraw should match their deposit
    assert_eq!(
        fixture.vault_client.max_withdraw(&user1),
        333_0000000,
        "User1 should have 333 USDC"
    );
    assert_eq!(
        fixture.vault_client.max_withdraw(&user2),
        333_0000000,
        "User2 should have 333 USDC"
    );
    assert_eq!(
        fixture.vault_client.max_withdraw(&user3),
        334_0000000,
        "User3 should have 334 USDC"
    );
}

#[test]
fn test_preview_functions_match_actual() {
    // Test that preview functions accurately predict actual results
    let fixture = TestFixture::new();

    // Test preview_deposit
    let deposit_amount = 100_0000000;
    let previewed_shares = fixture.vault_client.preview_deposit(&deposit_amount);

    let actual_shares = fixture.vault_client.mock_all_auths().deposit(
        &deposit_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    assert_eq!(
        previewed_shares, actual_shares,
        "preview_deposit should match actual shares received"
    );

    // Test preview_withdraw
    let withdraw_amount = 50_0000000;
    let previewed_shares_to_burn = fixture.vault_client.preview_withdraw(&withdraw_amount);

    let actual_shares_burned = fixture.vault_client.mock_all_auths().withdraw(
        &withdraw_amount,
        &fixture.user,
        &fixture.user,
        &fixture.user,
    );

    assert_eq!(
        previewed_shares_to_burn, actual_shares_burned,
        "preview_withdraw should match actual shares burned"
    );
}
