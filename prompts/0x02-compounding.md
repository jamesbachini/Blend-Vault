I need to claim BLND distribution tokens which are accumulating for the USDC position in the blend pool. BLND is a SEP41 token which is issued as an incentive to depositors into the blend pool. We need to claim it, sell it for USDC and then deposit that USDC back into the pool to create a auto-compounding effect.

There should be a public function to initiate this process independently. It should also be run prior to any withdrawal (with a safety net that if no emmissions are available it doesn't lock funds in the contract)

Step 1. Claim BLND

Pool contract ID: CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS

Example tx:
CCCC…GYFS claim(GAPL…FVG7, [3u32], GAPL…FVG7) → 35872773i128

Interface:
fn claim(from: address, reserve_token_ids: vec<u32>, to: address) -> i128

Step 2. Sell BLND for USDC

We will use the Comet BLND-USDC pool for this.

Contract ID: CAS3FL6TLZKDGGSISDBWGGPXT3NRR4DYTZD7YOD3HMYO6LTJUVGRVEAM


Interface:
fn swap_exact_amount_in(token_in: address, token_amount_in: i128, token_out: address, min_amount_out: i128, max_price: i128, user: address) -> tuple<i128,i128>

Step 3. Deposit USDC into the Blend Pool

Already have logic within the contract to do this.

--------------

Complete interface for the Comet BLND-USDC pool:CAS3FL6TLZKDGGSISDBWGGPXT3NRR4DYTZD7YOD3HMYO6LTJUVGRVEAM


fn init(controller: address, tokens: vec<address>, weights: vec<i128>, balances: vec<i128>, swap_fee: i128)

fn gulp(t: address)

fn join_pool(pool_amount_out: i128, max_amounts_in: vec<i128>, user: address)

fn exit_pool(pool_amount_in: i128, min_amounts_out: vec<i128>, user: address)

fn swap_exact_amount_in(token_in: address, token_amount_in: i128, token_out: address, min_amount_out: i128, max_price: i128, user: address) -> tuple<i128,i128>

fn swap_exact_amount_out(token_in: address, max_amount_in: i128, token_out: address, token_amount_out: i128, max_price: i128, user: address) -> tuple<i128,i128>

fn dep_tokn_amt_in_get_lp_tokns_out(token_in: address, token_amount_in: i128, min_pool_amount_out: i128, user: address) -> i128

fn dep_lp_tokn_amt_out_get_tokn_in(token_in: address, pool_amount_out: i128, max_amount_in: i128, user: address) -> i128

fn wdr_tokn_amt_in_get_lp_tokns_out(token_out: address, pool_amount_in: i128, min_amount_out: i128, user: address) -> i128

fn wdr_tokn_amt_out_get_lp_tokns_in(token_out: address, token_amount_out: i128, max_pool_amount_in: i128, user: address) -> i128

fn set_controller(manager: address)

fn set_freeze_status(val: bool)

fn get_total_supply() -> i128

fn get_controller() -> address

fn get_tokens() -> vec<address>

fn get_balance(token: address) -> i128

fn get_normalized_weight(token: address) -> i128

fn get_spot_price(token_in: address, token_out: address) -> i128

fn get_swap_fee() -> i128

fn get_spot_price_sans_fee(token_in: address, token_out: address) -> i128

fn allowance(from: address, spender: address) -> i128

fn approve(from: address, spender: address, amount: i128, expiration_ledger: u32)

fn balance(id: address) -> i128

fn transfer(from: address, to: address, amount: i128)

fn transfer_from(spender: address, from: address, to: address, amount: i128)

fn burn(from: address, amount: i128)

fn burn_from(spender: address, from: address, amount: i128)

fn decimals() -> u32

fn name() -> string

fn symbol() -> string

// STRUCTS

#[contracttype]
struct SwapEvent {
  caller: address,
  token_amount_in: i128,
  token_amount_out: i128,
  token_in: address,
  token_out: address
}

#[contracttype]
struct JoinEvent {
  caller: address,
  token_amount_in: i128,
  token_in: address
}

#[contracttype]
struct ExitEvent {
  caller: address,
  token_amount_out: i128,
  token_out: address
}

#[contracttype]
struct DepositEvent {
  caller: address,
  token_amount_in: i128,
  token_in: address
}

#[contracttype]
struct WithdrawEvent {
  caller: address,
  pool_amount_in: i128,
  token_amount_out: i128,
  token_out: address
}

#[contracttype]
struct Record {
  balance: i128,
  index: u32,
  scalar: i128,
  weight: i128
}

#[contracttype]
struct AllowanceDataKey {
  from: address,
  spender: address
}

#[contracttype]
struct AllowanceValue {
  amount: i128,
  expiration_ledger: u32
}

#[contracttype]
struct TokenMetadata {
  decimal: u32,
  name: string,
  symbol: string
}

// UNIONS

#[contracttype]
enum DataKey {
  Factory(),
  Controller(),
  SwapFee(),
  AllTokenVec(),
  AllRecordData(),
  TokenShare(),
  TotalShares(),
  PublicSwap(),
  Finalize(),
  Freeze()
}

#[contracttype]
enum DataKeyToken {
  Allowance(AllowanceDataKey),
  Balance(address),
  Nonce(address),
  State(address),
  Admin()
}

// ERRORS

#[contracterror]
enum Errors {
  ErrFinalized = 1,
  ErrNegative = 2,
  ErrMinFee = 3,
  ErrMaxFee = 4,
  ErrNotController = 5,
  ErrInvalidVectorLen = 6,
  AlreadyInitialized = 7,
  ErrIsBound = 8,
  ErrNotBound = 9,
  ErrMaxTokens = 10,
  ErrMinWeight = 11,
  ErrMaxWeight = 12,
  ErrMinBalance = 13,
  ErrFreezeOnlyWithdrawals = 14,
  ErrMinTokens = 15,
  ErrSwapFee = 16,
  ErrMaxInRatio = 17,
  ErrMathApprox = 18,
  ErrLimitIn = 19,
  ErrLimitOut = 20,
  ErrMaxOutRatio = 21,
  ErrBadLimitPrice = 22,
  ErrLimitPrice = 23,
  ErrTotalWeight = 24,
  ErrTokenAmountIsNegative = 25,
  ErrNotAuthorizedByAdmin = 26,
  ErrInsufficientAllowance = 27,
  ErrDeauthorized = 28,
  ErrInsufficientBalance = 29,
  ErrAddOverflow = 30,
  ErrSubUnderflow = 31,
  ErrDivInternal = 32,
  ErrMulOverflow = 33,
  ErrCPowBaseTooLow = 34,
  ErrCPowBaseTooHigh = 35,
  ErrInvalidExpirationLedger = 36,
  ErrNegativeOrZero = 37,
  ErrTokenInvalid = 38
}
