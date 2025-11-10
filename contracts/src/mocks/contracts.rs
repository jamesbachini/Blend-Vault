#![cfg(test)]

// Import WASM contracts from Blend's contract-sdk
// These contracts are used to create more realistic test environments

pub mod backstop {
    soroban_sdk::contractimport!(file = "./src/mocks/wasm/backstop.wasm");
}

pub mod emitter {
    soroban_sdk::contractimport!(file = "./src/mocks/wasm/emitter.wasm");
}

pub mod pool_factory {
    soroban_sdk::contractimport!(file = "./src/mocks/wasm/pool_factory.wasm");
}

pub mod pool {
    soroban_sdk::contractimport!(file = "./src/mocks/wasm/pool.wasm");
}

pub mod comet {
    soroban_sdk::contractimport!(file = "./src/mocks/wasm/comet.wasm");
}
