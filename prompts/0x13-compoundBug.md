There is still a bug in the compound function.

The frontend works and displays there is 0.0016 BLND to compound. However when I click compound I get a freighter confirmation window then it fails with an error returned from the RPC node (included at the end of this message)

Step 1. Please investigate the bug and create a fix.
Step 2. Update unit tests to check the fix actually works

Do not redeploy contracts, I will do this manually.

I really need to get a this function working, once a fix is in place could we test it with the real comet dex contracts:
https://github.com/CometDEX/comet-contracts-v1

Do not mock anything or simulate any logic, this needs to be production level code.


Notes and further information to aid with investigating the bug:
---------

On line 392 of contracts/src/lib.rs there is some code that does a transfer_from

I'm not sure this should be there. The comet pool itself should call transfer_from which it does in the pull_underlying function:
https://github.com/CometDEX/comet-contracts-v1/blob/ef4cbfad0a35202ad267c14d163d2f362995a8d3/contracts/src/c_pool/token_utility.rs#L13

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

This is called from the execute_swap_exact_amount_in function:
https://github.com/CometDEX/comet-contracts-v1/blob/ef4cbfad0a35202ad267c14d163d2f362995a8d3/contracts/src/c_pool/call_logic/pool.rs#L112

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

---------

Here is an example below for a smart contract that does autocompounding using soroswap. I DO NOT WANT TO USE SOROSWAP, swap still needs to take place on comet. But this might be useful as the rest of the logic is relevant.
https://github.com/paltalabs/defindex/blob/21b5cead2baaf8775be230719c4076365ad6a230/apps/contracts/strategies/blend/src/blend_pool.rs#L212

/// Reinvests BLND rewards back into the pool.
///
/// This function swaps BLND rewards for the underlying asset using a direct path
/// through the Soroswap router. The swapped assets are then supplied to the Blend
/// pool, and the strategy's reserves are updated accordingly.
///
/// # Process
/// 1. Check the BLND balance of the contract.
/// 2. If the balance is below the reward threshold, exit early.
/// 3. Swap BLND tokens for the underlying asset via Soroswap.
/// 4. Supply the swapped asset to the Blend pool.
/// 5. Update the strategy reserves to reflect the reinvested amount.
///
/// # Arguments
/// * `e` - The execution environment.
/// * `config` - The contract configuration containing asset and pool details.
///
/// # Returns
/// * `Result<StrategyReserves, StrategyError>` - Returns the strategy reserves if reinvestment was successful,
///   `false` if skipped due to low BLND balance, or an error if any step fails.
pub fn perform_reinvest(e: &Env, config: &Config, amount_out_min: i128) -> Result<StrategyReserves, StrategyError> {
    // Check the current BLND balance
    let blnd_balance =
        TokenClient::new(e, &config.blend_token).balance(&e.current_contract_address());

    // If balance does not exceed threshold, skip harvest
    if blnd_balance < config.reward_threshold {
        // get current reserves
        let reserves = reserves::get_strategy_reserve_updated(&e, &config);
        return Ok(reserves);
    }

    let swap_path = vec![e, config.blend_token.clone(), config.asset.clone()];

    let deadline = e
        .ledger()
        .timestamp()
        .checked_add(1)
        .ok_or( StrategyError::UnderflowOverflow)?;

    // Swapping BLND tokens to Underlying Asset
    let swapped_amounts = internal_swap_exact_tokens_for_tokens(
        e,
        &blnd_balance,
        &amount_out_min,
        swap_path,
        &e.current_contract_address(),
        &deadline,
        config,
    )?;
    let amount_out: i128 = swapped_amounts
        .get(1)
        .ok_or(StrategyError::InternalSwapError)?
        .into_val(e);

    // Supplying underlying asset into blend pool
    let b_tokens_minted = supply(&e, &e.current_contract_address(), &amount_out, &config, true)?;

    let reserves = reserves::harvest(&e, b_tokens_minted, &config)?;

    Ok(reserves)
}

---------

From the block explorer I can show what a successful swap looks like:
GB3J…NDBC invoked contract CAS3…VEAM swap_exact_amount_in(CD25…G5JY, 8649408230i128, CCW6…MI75, 424273725i128, 2006167560000000i128, GB3J…NDBC) → [431123610i128, 200632825i128]
 Invoked contract CAS3…VEAM swap_exact_amount_in(CD25…G5JY, 8649408230i128, CCW6…MI75, 424273725i128, 2006167560000000i128, GB3J…NDBC) → [431123610i128, 200632825i128]
 Invoked contract CD25…G5JY BLND[Blend] GDJE…EZYY approve(GB3J…NDBC, CAS3…VEAM, 8649408230i128, 59900000u32)
 Invoked contract CD25…G5JY BLND[Blend] GDJE…EZYY transfer_from(CAS3…VEAM, GB3J…NDBC, CAS3…VEAM, 8649408230i128)
 Invoked contract CCW6…MI75 USDCcentre.io transfer(CAS3…VEAM, GB3J…NDBC, 431123610i128)

---------

The full error I'm getting back from the compound function is:
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "latestLedger": 59817610,
        "latestLedgerCloseTime": "1762968095",
        "oldestLedger": 59696651,
        "oldestLedgerCloseTime": "1762269276",
        "status": "FAILED",
        "txHash": "d80242a47b049c63340bb6cc1b0ee035d81b6143af0ae81f86562ca32fae2d31",
        "applicationOrder": 531,
        "feeBump": false,
        "envelopeXdr": "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",
        "resultXdr": "AAAAAAAD++L/////AAAAAQAAAAAAAAAY/////gAAAAA=",
        "resultMetaXdr": "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",
        "diagnosticEventsXdr": [
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAADAAAADwAAAAdmbl9jYWxsAAAAAA0AAAAgQkDKAzRHGVmYvMZgVATCdhqe9SYL/vip7xCDatinQd4AAAAPAAAACGNvbXBvdW5kAAAAEgAAAAAAAAAAHrvcJ2VCqjXOcyOnFOaVmt6zBUxZlxBOn5CsdjXdDWI=",
            "AAAAAAAAAAAAAAABQkDKAzRHGVmYvMZgVATCdhqe9SYL/vip7xCDatinQd4AAAACAAAAAAAAAAMAAAAPAAAAB2ZuX2NhbGwAAAAADQAAACBCQMoDNEcZWZi8xmBUBMJ2Gp71Jgv++KnvEINq2KdB3gAAAA8AAAAMX19jaGVja19hdXRoAAAAEAAAAAEAAAADAAAADQAAACBuXFIabxic8srVQevkY7khEN4yACuPOkNBh5ZwprurIgAAAAEAAAAQAAAAAQAAAAEAAAAQAAAAAQAAAAIAAAAPAAAACENvbnRyYWN0AAAAEQAAAAEAAAADAAAADwAAAARhcmdzAAAAEAAAAAEAAAABAAAAEgAAAAAAAAAAHrvcJ2VCqjXOcyOnFOaVmt6zBUxZlxBOn5CsdjXdDWIAAAAPAAAACGNvbnRyYWN0AAAAEgAAAAFCQMoDNEcZWZi8xmBUBMJ2Gp71Jgv++KnvEINq2KdB3gAAAA8AAAAHZm5fbmFtZQAAAAAPAAAACGNvbXBvdW5k",
            "AAAAAAAAAAAAAAABQkDKAzRHGVmYvMZgVATCdhqe9SYL/vip7xCDatinQd4AAAACAAAAAAAAAAIAAAAPAAAABWVycm9yAAAAAAAAAgAAAAEAAAADAAAAEAAAAAEAAAACAAAADgAAAC90cnlpbmcgdG8gaW52b2tlIG5vbi1leGlzdGVudCBjb250cmFjdCBmdW5jdGlvbgAAAAAPAAAADF9fY2hlY2tfYXV0aA==",
            "AAAAAAAAAAAAAAABQkDKAzRHGVmYvMZgVATCdhqe9SYL/vip7xCDatinQd4AAAACAAAAAAAAAAIAAAAPAAAABWVycm9yAAAAAAAAAgAAAAkAAAAGAAAAEAAAAAEAAAADAAAADgAAAChmYWlsZWQgYWNjb3VudCBhdXRoZW50aWNhdGlvbiB3aXRoIGVycm9yAAAAEgAAAAFCQMoDNEcZWZi8xmBUBMJ2Gp71Jgv++KnvEINq2KdB3gAAAAIAAAABAAAAAw==",
            "AAAAAAAAAAAAAAABQkDKAzRHGVmYvMZgVATCdhqe9SYL/vip7xCDatinQd4AAAACAAAAAAAAAAIAAAAPAAAABWVycm9yAAAAAAAAAgAAAAkAAAAGAAAADgAAAEhlc2NhbGF0aW5nIGVycm9yIHRvIFZNIHRyYXAgZnJvbSBmYWlsZWQgaG9zdCBmdW5jdGlvbiBjYWxsOiByZXF1aXJlX2F1dGg=",
            "AAAAAAAAAAAAAAABQkDKAzRHGVmYvMZgVATCdhqe9SYL/vip7xCDatinQd4AAAACAAAAAAAAAAEAAAAPAAAAA2xvZwAAAAAQAAAAAQAAAAMAAAAOAAAAHlZNIGNhbGwgdHJhcHBlZCB3aXRoIEhvc3RFcnJvcgAAAAAADwAAAAhjb21wb3VuZAAAAAIAAAAJAAAABg==",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAA5ob3N0X2ZuX2ZhaWxlZAAAAAAAAgAAAAkAAAAGAAAAAQ==",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAACnJlYWRfZW50cnkAAAAAAAUAAAAAAAAAGw==",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAC3dyaXRlX2VudHJ5AAAAAAUAAAAAAAAAAA==",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAEGxlZGdlcl9yZWFkX2J5dGUAAAAFAAAAAAAAAJA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAEWxlZGdlcl93cml0ZV9ieXRlAAAAAAAABQAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAADXJlYWRfa2V5X2J5dGUAAAAAAAAFAAAAAAAAACg=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAADndyaXRlX2tleV9ieXRlAAAAAAAFAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAADnJlYWRfZGF0YV9ieXRlAAAAAAAFAAAAAAAAAJA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAD3dyaXRlX2RhdGFfYnl0ZQAAAAAFAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAADnJlYWRfY29kZV9ieXRlAAAAAAAFAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAD3dyaXRlX2NvZGVfYnl0ZQAAAAAFAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAACmVtaXRfZXZlbnQAAAAAAAUAAAAAAAAAAA==",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAD2VtaXRfZXZlbnRfYnl0ZQAAAAAFAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAACGNwdV9pbnNuAAAABQAAAAAAZY5w",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAACG1lbV9ieXRlAAAABQAAAAAAPcE/",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAEWludm9rZV90aW1lX25zZWNzAAAAAAAABQAAAAAABhTH",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAD21heF9yd19rZXlfYnl0ZQAAAAAFAAAAAAAAACg=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAEG1heF9yd19kYXRhX2J5dGUAAAAFAAAAAAAAAJA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAEG1heF9yd19jb2RlX2J5dGUAAAAFAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAgAAAAAAAAACAAAADwAAAAxjb3JlX21ldHJpY3MAAAAPAAAAE21heF9lbWl0X2V2ZW50X2J5dGUAAAAABQAAAAAAAAAA"
        ],
        "events": {
            "transactionEventsXdr": [
                "AAAAAAAAAAAAAAABJbT82FmuwvpjSEOMSJs8PBDJi20hvk/TyzDLaJU++XcAAAABAAAAAAAAAAIAAAAPAAAAA2ZlZQAAAAASAAAAAAAAAAAeu9wnZUKqNc5zI6cU5pWa3rMFTFmXEE6fkKx2Nd0NYgAAAAoAAAAAAAAAAAAAAAAADeV0",
                "AAAAAgAAAAAAAAABJbT82FmuwvpjSEOMSJs8PBDJi20hvk/TyzDLaJU++XcAAAABAAAAAAAAAAIAAAAPAAAAA2ZlZQAAAAASAAAAAAAAAAAeu9wnZUKqNc5zI6cU5pWa3rMFTFmXEE6fkKx2Nd0NYgAAAAr/////////////////9hZu"
            ]
        },
        "ledger": 59817610,
        "createdAt": "1762968095"
    }
}
