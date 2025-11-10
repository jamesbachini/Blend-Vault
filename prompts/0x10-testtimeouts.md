I've been getting timeouts in the unit tests so have used nextest to run longer tests.

cargo nextest run --target x86_64-unknown-linux-gnu --no-fail-fast

Here is the output, can you investigate why these tests are failing and make any bug fixes to the smart contract.

Summary [ 442.673s] 51 tests run: 47 passed (41 slow), 4 failed, 0 skipped
FAIL [ 180.855s] blend-vault test::test_compound_with_rewards_then_withdraw
FAIL [ 180.856s] blend-vault test::test_compound_with_rewards
FAIL [  79.524s] blend-vault test::test_zero_deposit
FAIL [  78.945s] blend-vault test::test_zero_mint

thread 'test::test_zero_mint' (14001) panicked at host.rs:861:9:
    HostError: Error(Contract, #1216)

    Event log (newest first):
       0: [Diagnostic Event] topics:[error, Error(Contract, #1216)], data:"escalating error to panic"
       1: [Diagnostic Event] topics:[error, Error(Contract, #1216)], data:["contract call failed", mint, [0, CAA...AFCT4, CAA...AFCT4, CAA...AFCT4]]
       2: [Failed Diagnostic Event (not emitted)] contract:CAA...AVAX5, topics:[error, Error(Contract, #1216)], data:"caught error from function"
       3: [Failed Diagnostic Event (not emitted)] contract:CAA...AVAX5, topics:[error, Error(Contract, #1216)], data:"escalating error to panic"
       4: [Failed Diagnostic Event (not emitted)] contract:CAA...AVAX5, topics:[error, Error(Contract, #1216)], data:["contract call failed", submit_with_allowance, [CAA...AVAX5, CAA...AVAX5, CAA...AVAX5, [{address: CAJX...R7Q7P, amount: 0, request_type: 2}]]]


thread 'test::test_zero_deposit' (13999) panicked at host.rs:861:9:
    HostError: Error(Contract, #1216)

    Event log (newest first):
       0: [Diagnostic Event] topics:[error, Error(Contract, #1216)], data:"escalating error to panic"
       1: [Diagnostic Event] topics:[error, Error(Contract, #1216)], data:["contract call failed", deposit, [0, CAA...AFCT4, CAA...AFCT4, CAA...AFCT4]]
       2: [Failed Diagnostic Event (not emitted)] contract:CAA...AVAX5, topics:[error, Error(Contract, #1216)], data:"caught error from function"
       3: [Failed Diagnostic Event (not emitted)] contract:CAA...AVAX5, topics:[error, Error(Contract, #1216)], data:"escalating error to panic"
       4: [Failed Diagnostic Event (not emitted)] contract:CAA...AVAX5, topics:[error, Error(Contract, #1216)], data:["contract call failed", submit_with_allowance, [CAA...AVAX5, CAA...AVAX5, CAA...AVAX5, [{address: CAJX...R7Q7P, amount: 0, request_type: 2}]]]

thread 'test::test_compound_with_rewards' (13921) panicked at host.rs:861:9:
    HostError: Error(Contract, #1000)

    Event log (newest first):
       0: [Diagnostic Event] topics:[error, Error(Contract, #1000)], data:"escalating error to panic"
       1: [Diagnostic Event] topics:[error, Error(Contract, #1000)], data:["contract call failed", distribute, []]
       2: [Failed Diagnostic Event (not emitted)] contract:CAA...AMDR4, topics:[log], data:["VM call trapped with HostError", distribute, Error(Contract, #1000)]
       3: [Failed Diagnostic Event (not emitted)] contract:CAA...AMDR4, topics:[error, Error(Contract, #1000)], data:"escalating error to VM trap from failed host function call: fail_with_error"
       4: [Failed Diagnostic Event (not emitted)] contract:CAA...AMDR4, topics:[error, Error(Contract, #1000)], data:["failing with contract error", 1000]

thread 'test::test_compound_with_rewards_then_withdraw' (13936) panicked at host.rs:861:9:
    HostError: Error(Contract, #1000)

    Event log (newest first):
       0: [Diagnostic Event] topics:[error, Error(Contract, #1000)], data:"escalating error to panic"
       1: [Diagnostic Event] topics:[error, Error(Contract, #1000)], data:["contract call failed", distribute, []]
       2: [Failed Diagnostic Event (not emitted)] contract:CAA...AMDR4, topics:[log], data:["VM call trapped with HostError", distribute, Error(Contract, #1000)]
       3: [Failed Diagnostic Event (not emitted)] contract:CAA...AMDR4, topics:[error, Error(Contract, #1000)], data:"escalating error to VM trap from failed host function call: fail_with_error"
       4: [Failed Diagnostic Event (not emitted)] contract:CAA...AMDR4, topics:[error, Error(Contract, #1000)], data:["failing with contract error", 1000]

Continue your work to attempt to wire the entire suite (and the compound tests in particular) to the real Blend WASMs revealed a number of integration details we still need to resolve (emission configuration, ledger scheduling, claim token IDs).

Finish this task by ensuring the contract is production ready, fully tested, robust and safe to deploy to Stellar mainnet.