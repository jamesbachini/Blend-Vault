Please create a token vault yield strategy for Stellar using a Soroban smart contract in ./contracts/src/lib.rs

The strategy should accept USDC deposits and then funds should be deposited to BlendProtocols Yield Box Pool v2.

https://mainnet.blend.capital/dashboard/?poolId=CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS

The contract should deposit any USDC that is put into the vault into Blend's pool and then issue the user a token. Then when the user withdraws funds it will need to redeem the right amount from the Blend vault and pay the user back with interest or the correct percentage share of the growing pool.

I have the cargo crate blend-contract-sdk installed which should make this easier. Here are the specific versions of crates that I want to use (DO NOT DOWNGRADE FOR COMPATIBILITY):

soroban-sdk = "23.1.0"
sep-41-token = "1.3.1"
blend-contract-sdk = 2.22.0"
stellar-tokens = "0.5.0"
stellar-access = "0.5.0"
stellar-contract-utils = "0.5.0"
stellar-macros = "0.5.0"

Here is an example using that blend-contract-sdk crate:

use soroban_sdk::{symbol_short, testutils::{Address as _, BytesN as _}, Address, BytesN, Env};

use blend_contract_sdk::{pool, testutils::{default_reserve_config, BlendFixture}};

let env = Env::default();
let deployer = Address::generate(&env);
let blnd = env.register_stellar_asset_contract_v2(deployer.clone()).address();
let usdc = env.register_stellar_asset_contract_v2(deployer.clone()).address();
let blend = BlendFixture::deploy(&env, &deployer, &blnd, &usdc);

let token = env.register_stellar_asset_contract_v2(deployer.clone()).address();
let pool = blend.pool_factory.mock_all_auths().deploy(
    &deployer,
    &symbol_short!("test"),
    &BytesN::<32>::random(&env),
    &Address::generate(&env),
    &0_1000000, // 10%
    &4, // 4 max positions
);
let pool_client = pool::Client::new(&env, &pool);
let reserve_config = default_reserve_config();
pool_client.mock_all_auths().queue_set_reserve(&token, &reserve_config);
pool_client.mock_all_auths().set_reserve(&token);

blend.backstop.mock_all_auths().deposit(&deployer, &pool, &50_000_0000000);
pool_client.mock_all_auths().set_status(&3); // remove pool from setup status
pool_client.mock_all_auths().update_status(); // update status based on backstop

Here is an example vault contract using the new OpenZeppelin libraries which are also installed:

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::fungible::{
    vault::{FungibleVault, Vault},
    Base, FungibleToken,
};

#[contract]
pub struct VaultExampleContract;

#[contractimpl]
impl VaultExampleContract {
    // Constructor to initialize the vault
    pub fn __constructor(e: &Env, asset: Address, decimals_offset: u32) {
        // Set the underlying asset and the decimal offset once, at initialization.
        Vault::set_asset(e, asset);
        Vault::set_decimals_offset(e, decimals_offset);
        // Initialize metadata (name, symbol, decimals) for the share token.
        // We call Self::decimals(e) to get underlying_decimals + offset.
        Base::set_metadata(
            e,
            Self::decimals(e),
            String::from_str(e, "Vault Token"),
            String::from_str(e, "VLT"),
        );
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for VaultExampleContract {
    type ContractType = Vault;

    // Override the decimals function (demonstration).
    fn decimals(e: &Env) -> u32 {
        Vault::decimals(e)
    }

    // We could override other Base token functions if needed, but by default
    // default_impl macro will use the default implementations.
}

#[contractimpl]
impl FungibleVault for VaultExampleContract {
    // We delegate each vault-specific function to the standard Vault implementation.
    fn query_asset(e: &Env) -> Address {
        Vault::query_asset(e)
    }

    fn total_assets(e: &Env) -> i128 {
        Vault::total_assets(e)
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

    fn deposit(e: &Env, assets: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        operator.require_auth();
        Vault::deposit(e, assets, receiver, from, operator)
    }

    fn mint(e: &Env, shares: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        operator.require_auth();
        Vault::mint(e, shares, receiver, from, operator)
    }

    fn withdraw(
        e: &Env,
        assets: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        operator.require_auth();
        Vault::withdraw(e, assets, receiver, owner, operator)
    }

    fn redeem(e: &Env, shares: i128, receiver: Address, owner: Address, operator: Address) -> i128 {
        operator.require_auth();
        Vault::redeem(e, shares, receiver, owner, operator)
    }
}

Here is the token interface for Blend's pool contract: CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS

// RUST version: 1.81.0
// SDK version: 22.0.7#211569aa49c8d896877dfca1f2eb4fe9071121c8

// FUNCTIONS

/// Initialize the pool
/// 
/// ### Arguments
/// Creator supplied:
/// * `admin` - The Address for the admin
/// * `name` - The name of the pool
/// * `oracle` - The contract address of the oracle
/// * `backstop_take_rate` - The take rate for the backstop (7 decimals)
/// * `max_positions` - The maximum number of positions a user is permitted to have
/// * `min_collateral` - The minimum collateral required to open a borrow position in the oracles base asset
/// 
/// Pool Factory supplied:
/// * `backstop_id` - The contract address of the pool's backstop module
/// * `blnd_id` - The contract ID of the BLND token
fn __constructor(admin: address, name: string, oracle: address, bstop_rate: u32, max_positions: u32, min_collateral: i128, backstop_id: address, blnd_id: address)

fn propose_admin(new_admin: address)

fn accept_admin()

fn update_pool(backstop_take_rate: u32, max_positions: u32, min_collateral: i128)

fn queue_set_reserve(asset: address, metadata: ReserveConfig)

fn cancel_set_reserve(asset: address)

fn set_reserve(asset: address) -> u32

fn get_config() -> PoolConfig

fn get_admin() -> address

fn get_reserve_list() -> vec<address>

fn get_reserve(asset: address) -> Reserve

fn get_positions(address: address) -> Positions

fn submit(from: address, spender: address, to: address, requests: vec<Request>) -> Positions

fn submit_with_allowance(from: address, spender: address, to: address, requests: vec<Request>) -> Positions

fn flash_loan(from: address, flash_loan: FlashLoan, requests: vec<Request>) -> Positions

fn update_status() -> u32

fn set_status(pool_status: u32)

fn gulp(asset: address) -> i128

fn gulp_emissions() -> i128

fn set_emissions_config(res_emission_metadata: vec<ReserveEmissionMetadata>)

fn claim(from: address, reserve_token_ids: vec<u32>, to: address) -> i128

fn get_reserve_emissions(reserve_token_index: u32) -> option<ReserveEmissionData>

fn get_user_emissions(user: address, reserve_token_index: u32) -> option<UserEmissionData>

fn new_auction(auction_type: u32, user: address, bid: vec<address>, lot: vec<address>, percent: u32) -> AuctionData

fn get_auction(auction_type: u32, user: address) -> AuctionData

fn del_auction(auction_type: u32, user: address)

fn bad_debt(user: address)

// STRUCTS

#[contracttype]
struct AuctionData {
  /// A map of the assets being bid on and the amount being bid. These are tokens spent
  /// by the filler of the auction.
  /// 
  /// The bid is different based on each auction type:
  /// - UserLiquidation: dTokens
  /// - BadDebtAuction: dTokens
  /// - InterestAuction: Underlying assets (backstop token)
  bid: map<address,i128>,
  /// The block the auction begins on. This is used to determine how the auction
  /// should be scaled based on the number of blocks that have passed since the auction began.
  block: u32,
  /// A map of the assets being auctioned off and the amount being auctioned. These are tokens
  /// received by the filler of the auction.
  /// 
  /// The lot is different based on each auction type:
  /// - UserLiquidation: bTokens
  /// - BadDebtAuction: Underlying assets (backstop token)
  /// - InterestAuction: Underlying assets
  lot: map<address,i128>
}

/// Metadata for a pool's reserve emission configuration
#[contracttype]
struct ReserveEmissionMetadata {
  res_index: u32,0
  res_type: u32,
  share: u64
}

/// A request a user makes against the pool
#[contracttype]
struct Request {
  address: address,
  amount: i128,
  request_type: u32
}

#[contracttype]
struct FlashLoan {
  amount: i128,
  asset: address,
  contract: address
}

#[contracttype]
struct Reserve {
  asset: address,
  config: ReserveConfig,
  data: ReserveData,
  scalar: i128
}

/// A user / contracts position's with the pool, stored in the Reserve's decimals
#[contracttype]
struct Positions {
  collateral: map<u32,i128>,
  liabilities: map<u32,i128>,
  supply: map<u32,i128>
}

/// The pool's config
#[contracttype]
struct PoolConfig {
  bstop_rate: u32,
  max_positions: u32,
  min_collateral: i128,
  oracle: address,
  status: u32
}

/// The pool's emission config
#[contracttype]
struct PoolEmissionConfig {
  config: u128,
  last_time: u64
}

/// The configuration information about a reserve asset
#[contracttype]
struct ReserveConfig {
  c_factor: u32,
  decimals: u32,
  enabled: bool,
  index: u32,
  l_factor: u32,
  max_util: u32,
  r_base: u32,
  r_one: u32,
  r_three: u32,
  r_two: u32,
  reactivity: u32,
  supply_cap: i128,
  util: u32
}

#[contracttype]
struct QueuedReserveInit {
  new_config: ReserveConfig,
  unlock_time: u64
}

/// The data for a reserve asset
#[contracttype]
struct ReserveData {
  b_rate: i128,
  b_supply: i128,
  backstop_credit: i128,
  d_rate: i128,
  d_supply: i128,
  ir_mod: i128,
  last_time: u64
}

/// The emission data for the reserve b or d token
#[contracttype]
struct ReserveEmissionData {
  eps: u64,
  expiration: u64,
  index: i128,
  last_time: u64
}

/// The user emission data for the reserve b or d token
#[contracttype]
struct UserEmissionData {
  accrued: i128,
  index: i128
}

#[contracttype]
struct UserReserveKey {
  reserve_id: u32,
  user: address
}

#[contracttype]
struct AuctionKey {
  auct_type: u32,
  user: address
}

/// Price data for an asset at a specific timestamp
#[contracttype]
struct PriceData {
  price: i128,
  timestamp: u64
}

// UNIONS

#[contracttype]
enum PoolDataKey {
  ResConfig(address),
  ResInit(address),
  ResData(address),
  EmisData(u32),
  Positions(address),
  UserEmis(UserReserveKey),
  Auction(AuctionKey)
}

/// Asset type
#[contracttype]
enum Asset {
  Stellar(address),
  Other(symbol)
}

// ERRORS

#[contracterror]
enum Errors {
  InternalError = 1,
  AlreadyInitializedError = 3,
  UnauthorizedError = 4,
  NegativeAmountError = 8,
  BalanceError = 10,
  OverflowError = 12,
  BadRequest = 1200,
  InvalidPoolConfigArgs = 1201,
  InvalidReserveMetadata = 1202,
  InitNotUnlocked = 1203,
  StatusNotAllowed = 1204,
  InvalidHf = 1205,
  InvalidPoolStatus = 1206,
  InvalidUtilRate = 1207,
  MaxPositionsExceeded = 1208,
  InternalReserveNotFound = 1209,
  InvalidPrice = 1210,
  InvalidLiquidation = 1211,
  AuctionInProgress = 1212,
  InvalidLiqTooLarge = 1213,
  InvalidLiqTooSmall = 1214,
  InterestTooSmall = 1215,
  InvalidBTokenMintAmount = 1216,
  InvalidBTokenBurnAmount = 1217,
  InvalidDTokenMintAmount = 1218,
  InvalidDTokenBurnAmount = 1219,
  ExceededSupplyCap = 1220,
  InvalidBid = 1221,
  InvalidLot = 1222,
  ReserveDisabled = 1223,
  MinCollateralNotMet = 1224
}
