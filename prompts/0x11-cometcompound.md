I've deployed the contract to mainnet and the deposit, withdraw functions work correctly.

I still have an issue when calling compound(operator), I'm using my own address for this which isn't the deployer address but I don't think we need that operator variable in the compound function do we? or try_compound?

That's not why it's failing though. There is a bug somewhere which is preventing swap_exact_amount_in to function correctly.

There is some code in the contract that is probably creating a simulated environment in testing to get around the underlying bug so unit tests pass. This should be removed: line 370 "Authorize the upcoming swap call so the vault can satisfy Comet's auth checks"

Find out what is causing the issue, create a fix. Do thorough testing to make sure the contract is ready for deployment and we can be confident it will work this time.

Here is the RPC error sent back when calling compound(callerAddress)

HostError: Error(WasmVm, InvalidAction)
Event log (newest first):
   0: [Diagnostic Event] contract:CDNEXS...FXNB, topics:[error, Error(WasmVm, InvalidAction)], data:"escalating error to VM trap from failed host function call: call"
   1: [Diagnostic Event] contract:CDNEXS...FXNB, topics:[error, Error(WasmVm, InvalidAction)], data:["contract call failed", swap_exact_amount_in, [CD25...G5JY, 8, CCW6...MI75, 0, 170141183460469231731687303715884105727, CDNEXS...FXNB]]
   2: [Failed Diagnostic Event (not emitted)] contract:CAS3...VEAM, topics:[error, Error(WasmVm, InvalidAction)], data:["VM call trapped: UnreachableCodeReached", swap_exact_amount_in]
   3: [Diagnostic Event] contract:CDNEXS...FXNB, topics:[fn_call, CAS3...VEAM, swap_exact_amount_in], data:[CD25...G5JY, 8, CCW6...MI75, 0, 170141183460469231731687303715884105727, CDNEXS...FXNB]
   4: [Diagnostic Event] contract:CD25...G5JY, topics:[fn_return, approve], data:Void
   5: [Contract Event] contract:CD25...G5JY, topics:[approve, CDNEXS...FXNB, CAS3...VEAM, "BLND:GDJ...EZYY"], data:[8, 59910677]

Here are some successful swaps from the contract history if that helps:
swap_exact_amount_in(
 token_in: CD25…G5JY,
 token_amount_in: 20031614006i128,
 token_out: CCW6…MI75,
 min_amount_out: 1i128,
 max_price: 170141183460469231731687303715884105727i128,
 user: CC2J…BR3Y) → [
1000893848i128,
200155815i128
]

swap_exact_amount_in(
 token_in: CD25…G5JY,
 token_amount_in: 16693417715i128,
 token_out: CCW6…MI75,
 min_amount_out: 818386913i128,
 max_price: 2007231070000000i128,
 user: GB3J…NDBC) → [
831839643i128,
200696236i128
]

Here is the source code for the comet dex pool if that helps:

pub fn swap_exact_amount_in(
   e: Env,
   token_in: Address,
   token_amount_in: i128,
   token_out: Address,
   min_amount_out: i128,
   max_price: i128,
   user: Address,
) -> (i128, i128) {
   user.require_auth();
   e.storage()
      .instance()
      .extend_ttl(SHARED_LIFETIME_THRESHOLD, SHARED_BUMP_AMOUNT);
   execute_swap_exact_amount_in(
      e,
      token_in,
      token_amount_in,
      token_out,
      min_amount_out,
      max_price,
      user,
   )
}

pub fn execute_swap_exact_amount_in(
    e: Env,
    token_in: Address,
    token_amount_in: i128,
    token_out: Address,
    min_amount_out: i128,
    max_price: i128,
    user: Address,
) -> (i128, i128) {
    assert_with_error!(&e, !read_freeze(&e), Error::ErrFreezeOnlyWithdrawals);
    assert_with_error!(&e, token_amount_in > 0, Error::ErrNegativeOrZero);
    assert_with_error!(&e, min_amount_out >= 0, Error::ErrNegative);
    assert_with_error!(&e, max_price >= 0, Error::ErrNegative);

    let swap_fee = read_swap_fee(&e);
    let mut record_map = read_record(&e);
    let mut in_record = record_map
        .get(token_in.clone())
        .unwrap_or_else(|| panic_with_error!(&e, Error::ErrNotBound));
    let mut out_record = record_map
        .get(token_out.clone())
        .unwrap_or_else(|| panic_with_error!(&e, Error::ErrNotBound));
    assert_with_error!(
        &e,
        token_amount_in
            <= in_record
                .balance
                .fixed_mul_floor(MAX_IN_RATIO, STROOP)
                .unwrap_optimized(),
        Error::ErrMaxInRatio
    );

    let spot_price_before = c_math::calc_spot_price(&in_record, &out_record, swap_fee);

    assert_with_error!(&e, spot_price_before <= max_price, Error::ErrBadLimitPrice);
    let token_amount_out = c_math::calc_token_out_given_token_in(
        &e,
        &in_record,
        &out_record,
        token_amount_in,
        swap_fee,
    );
    assert_with_error!(&e, token_amount_out >= min_amount_out, Error::ErrLimitOut);

    in_record.balance = in_record
        .balance
        .checked_add(token_amount_in)
        .unwrap_optimized();
    assert_with_error!(
        &e,
        out_record.balance >= token_amount_out,
        Error::ErrInsufficientBalance
    );
    out_record.balance = out_record.balance - token_amount_out;

    let spot_price_after = c_math::calc_spot_price(&in_record, &out_record, swap_fee);

    assert_with_error!(
        &e,
        spot_price_after >= spot_price_before,
        Error::ErrMathApprox
    );
    assert_with_error!(&e, spot_price_after <= max_price, Error::ErrLimitPrice);
    assert_with_error!(
        &e,
        spot_price_before
            <= token_amount_in
                .fixed_div_floor(token_amount_out, STROOP)
                .unwrap_optimized(),
        Error::ErrMathApprox
    );

    let event: SwapEvent = SwapEvent {
        caller: user.clone(),
        token_in: token_in.clone(),
        token_out: token_out.clone(),
        token_amount_in,
        token_amount_out,
    };
    e.events().publish((POOL, symbol_short!("swap")), event);

    pull_underlying(
        &e,
        &token_in,
        &user,
        token_amount_in,
        token_amount_in.clone(),
    );
    push_underlying(&e, &token_out, &user, token_amount_out);

    record_map.set(token_in, in_record);
    record_map.set(token_out, out_record);

    write_record(&e, record_map);

    (token_amount_out, spot_price_after)
}

pub fn pull_underlying(e: &Env, token: &Address, from: &Address, amount: i128, max_amount: i128) {
    // @DEV - This rounds the sequence number to the nearest 100000 to avoid simulation -> execution sequence number mismatch
    let ledger = (e.ledger().sequence() / 100000 + 1) * 100000;
    Client::new(e, token).approve(&from, &e.current_contract_address(), &max_amount, &ledger);
    Client::new(e, token).transfer_from(
        &e.current_contract_address(),
        &from,
        &e.current_contract_address(),
        &amount,
    );
}

// Transfers the Specific Token from the Contract’s Address to the given 'to' Address
pub fn push_underlying(e: &Env, token: &Address, to: &Address, amount: i128) {
    Client::new(e, token).transfer(&e.current_contract_address(), &to, &amount);
}