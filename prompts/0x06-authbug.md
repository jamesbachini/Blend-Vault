The new unit tests in contracts/ that check the spend authorization works are failing. We've had multiple issues with this and I want to test them on a local environment using the blend-contract-sdk to create a mock pool that behaves as the real pool on mainnet will.

Tests should be run with:
cargo test --target x86_64-unknown-linux-gnu

test test::test_deposit_with_authorization ... FAILED
test test::test_multiple_deposits_with_auth ... FAILED
test test::test_withdraw_with_authorization ... FAILED

Can you do a deep dive into this and find out what the underlying issue is. We've already tried submit() and submit_with_authorization()

When using the frontend I can authorize spend on USDC and then it fails when clicking deposit.

Additional info from blends docs:

The Lending pool contract allows users and liquidators to manipulate the user funds it stores.

Requests
All fund management is carried out using Request structs

pub struct Request {
    pub request_type: u32,
    pub address: Address, // asset or liquidatee address
    pub amount: i128,
}

which are input into a single submit() function. Multiple requests can be bundled together to carry out actions atomically (e.g. supply and borrow in the same transaction). The submit() function will simply revert if the user's account is unhealthy after all requests are processed.

The following request types are supported:

Deposit (enum 0): Deposits funds into the pool. These funds are not collateralized.

This request is useful for users who want to deposit funds but do not want these funds to be liquidated in the event their account becomes delinquent. Additionally, they're valuable in pools with strict position count limits since uncollateralized deposits don't count toward position count limits.

Deposit requests will fail if the pool status is greater than 3 (this means the pool is Frozen)

Withdraw (enum 1): Withdraws uncollateralized funds from the pool.

Deposit Collateral (enum 2): Deposits collateral into the pool.

Deposit Collateral requests will fail if the pool status is greater than 3 (this means the pool is Frozen)

Withdraw Collateral (enum 3): Withdraws collateral from the pool.

Borrow (enum 4): Borrows funds from the pool.

Borrow requests will fail if pool_status is greater than 1 (meaning the pool is Frozen or On-Ice)

Repay (enum 5): Repays borrowed funds.

Fill Liquidation (enum 6): Fills a user liquidation. This involves transferring a portion of the liquidated user's collateral and liabilities to the liquidator.

Fill Bad Debt Auction (enum 7): Fills a bad debt auction. This involves transferring bad debt stored as liabilities in the Backstop contract's positions to the liquidator's positions.

Fill Interest Auction (enum 8): Fills an interest auction. This request does not modify the filler's positions.

Delete Liquidation Auction (enum 9): Cancel's an ongoing liquidation.

Delete Liquidation Auction requests will fail if pool_status is greater than 1 (meaning the pool is Frozen or On-Ice)

All requests will fail if the pool status is 6 (Setup)

Requests are flexible in that they can be carried out on behalf of other users utilizing the spender from and to parameters on the submit() function. This allows users to delegate fund management to other users or contracts.

The addresses input into the from and to parameters are required to authorize the submit() call or it will fail.

Additional Submit Methods
User's can also utilize submit_with_allowance()  and flash_loan() to modify positions. Submit with allowance prompts the pool contract to call transfer_from() instead of transfer()when moving tokens, this is required for some integrations. flash_loan() allows users to borrow as much as they want from the pool without posting collateral as long as their position is healthy at the end of modification. This is useful for arbitrage bots, liquidation bots, and for easily entering leveraged positions.

The source code for the submit function is here: https://github.com/blend-capital/blend-contracts/blob/main/pool/src/pool/submit.rs