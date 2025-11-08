#!/usr/bin/env node

const StellarSdk = require('@stellar/stellar-sdk');

const VAULT_CONTRACT_ID = 'CDGSSWMTWASCHMRPPWZ2YQJOSARYJRIFJDNNA2COB35BXACESZE7RXHQ';
const SOROBAN_RPC_URL = 'https://rpc.lightsail.network/';
const NETWORK_PASSPHRASE = StellarSdk.Networks.PUBLIC;

// Get user address from command line
const userAddress = process.argv[2];

if (!userAddress) {
  console.error('Usage: node test-balance.js <USER_ADDRESS>');
  process.exit(1);
}

async function testBalance() {
  const server = new StellarSdk.rpc.Server(SOROBAN_RPC_URL);

  console.log('Testing vault balance for:', userAddress);
  console.log('Vault contract:', VAULT_CONTRACT_ID);
  console.log('---');

  try {
    // Test 1: Call balance method
    console.log('\n1. Calling balance() method:');
    const vaultContract = new StellarSdk.Contract(VAULT_CONTRACT_ID);
    const addressScVal = StellarSdk.nativeToScVal(userAddress, { type: 'address' });

    // We need a source account for simulation even though we don't sign
    const sourceKeypair = StellarSdk.Keypair.random();
    const sourceAccount = new StellarSdk.Account(sourceKeypair.publicKey(), '0');

    const tx = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(vaultContract.call('balance', addressScVal))
      .setTimeout(180)
      .build();

    const simResult = await server.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(simResult)) {
      const balance = StellarSdk.scValToBigInt(simResult.result.retval);
      console.log('  Share balance:', balance.toString());

      // If balance > 0, test convert_to_assets
      if (balance > 0n) {
        console.log('\n2. Converting shares to assets:');
        const sharesScVal = StellarSdk.nativeToScVal(balance, { type: 'i128' });

        const tx2 = new StellarSdk.TransactionBuilder(sourceAccount, {
          fee: StellarSdk.BASE_FEE,
          networkPassphrase: NETWORK_PASSPHRASE,
        })
          .addOperation(vaultContract.call('convert_to_assets', sharesScVal))
          .setTimeout(180)
          .build();

        const simResult2 = await server.simulateTransaction(tx2);

        if (StellarSdk.rpc.Api.isSimulationSuccess(simResult2)) {
          const assets = StellarSdk.scValToBigInt(simResult2.result.retval);
          console.log('  USDC value:', (Number(assets) / 1e7).toFixed(7), 'USDC');
        } else {
          console.log('  Error:', simResult2.error);
        }
      } else {
        console.log('\n  No shares found. User has not deposited or shares were burned.');
      }
    } else {
      console.log('  Simulation error:', simResult.error);
    }

    // Test 2: Try to read storage directly (diagnostic)
    console.log('\n3. Checking contract storage:');
    try {
      const storageKey = StellarSdk.xdr.ScVal.scvVec([
        StellarSdk.xdr.ScVal.scvSymbol('Balance'),
        addressScVal,
      ]);

      const result = await server.getContractData(VAULT_CONTRACT_ID, storageKey);

      if (result && result.val) {
        const ledgerData = result.val;
        const scVal = ledgerData.contractData().val();
        const storageBalance = StellarSdk.scValToBigInt(scVal);
        console.log('  Direct storage read:', storageBalance.toString());
      }
    } catch (e) {
      console.log('  Storage key not found or error:', e.message);
    }

  } catch (error) {
    console.error('\nError:', error.message);
    if (error.response) {
      console.error('Response:', JSON.stringify(error.response.data, null, 2));
    }
  }
}

testBalance();
