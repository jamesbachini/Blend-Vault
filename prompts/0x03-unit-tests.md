Please create unit tests for this contract. I would like a full comprehensive suite of unit tests.

Step 1. Setup testing environment with mock tokens and pools, the comet BLND-USDC pool might need a custom mock interface created as we can't deploy a local dex.

Step 2. Create a full suite of unit tests for this contract to test individual functions, common flows and edge cases to search for logic errors and bugs.

Step 3. Run the unit tests and investigate any issues raised by failed tests. Do not delete any tests that fail. Investigate and fix the underlying bugs in the contract. Use this unit testing framework to ensure the contract in contracts/src/lib.rs is robust and ready for production.

We can use the following cargo crate to create test USDC:
sep-41-token = { version = "1.3.1", features = ["testutils"] }

Example code:
use sep_41_token::testutils::{MockTokenClient, MockTokenWASM};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

let env = Env::default();

let admin = Address::generate(&env);
let token_id = env.register_contract_wasm(None, MockTokenWASM);
let token_client = MockTokenClient::new(&env, &token_id);
token_client.initialize(
    &admin,
    &7,
    &String::from_str(&env, "Name"),
    &String::from_str(&env, "Symbol"),
);

To create a mock blend pool we may need to use the blend-contract-sdk.

The latest version is 2.22.0 wheras we are using soroban-sdk = "23.1.0" which means it might have compatibility issues. Try it and see it works with soroban-sdk v23. I DO NOT WANT TO DOWNGRADE ANY EXISTING CRATES - DO NOT USE soroban-sdk v22!!

# Information on the cargo crate blend-contract-sdk

Blend Contract SDK
This repository contains interfaces, clients, and WASM blobs for the Blend Protocol as implemented in the Blend Contracts repository.

Documentation
To learn more about the Blend Protocol, visit the the docs:

Blend Docs
Versioning
The Blend Contract SDK uses a modified versioning system to control what version of the Stellar protocol the package supports.

[blend-protocol-version].[stellar-protocol-version].[sdk-version]

IE - 1.22.0 is the first version of the contract-sdk with v1 Blend Contracts and the protocol 22 Soroban SDK.

Modules
The Blend Contract SDK generates modules from the contractimport Soroban SDK macro. Each module exposes a Client, WASM, and the respective types needed to interact with the Blend Protocol. The following Blend contracts are exposed as a module:

backstop - Contract import for the backstop contract
emitter- Contract import for the emitter contract
pool - Contract import for the pool contract
pool_factory - Contract import for the pool factory contract
Testing (testutils)
External Dependencies
The Blend Contract SDK includes contractimport's of the Comet Contracts when compiled for test purposes via the testutils feature.

This includes:

comet - Contract import for the comet pool contract
comet_factory - Contract import for the comet pool factory contract
NOTE: These contracts were used for testing the Blend Protocol and should not be considered to be the latest version of the Comet Protocol. Please verify any non-test usage of the Comet contracts against the Comet GitHub.

Setup
The testutils module allows for easy deployment of Blend Contracts to be used in a unit test. The following example shows how to use the testutils to deploy a set of Blend Contracts and set up a pool.

If you require using the pool, please look at the following sep-41-oracle crate to deploy a mock oracle contract:

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