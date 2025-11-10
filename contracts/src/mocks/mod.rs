#![cfg(test)]

pub mod simple_mocks;

// Re-export commonly used items for convenience
pub use blend_contract_sdk::testutils::{default_reserve_config, BlendFixture};
pub use simple_mocks::{
    MockBlendPool, MockBlendPoolClient, MockCometPool, RealisticMockBlendPool,
    RealisticMockBlendPoolClient,
};
