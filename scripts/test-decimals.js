#!/usr/bin/env node

const StellarSdk = require('@stellar/stellar-sdk');

const VAULT_CONTRACT_ID = 'CDGSSWMTWASCHMRPPWZ2YQJOSARYJRIFJDNNA2COB35BXACESZE7RXHQ';
const USDC_CONTRACT_ID = 'CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75';
const SOROBAN_RPC_URL = 'https://rpc.lightsail.network/';
const NETWORK_PASSPHRASE = StellarSdk.Networks.PUBLIC;

async function testDecimals() {
  const server = new StellarSdk.rpc.Server(SOROBAN_RPC_URL);
  const vaultContract = new StellarSdk.Contract(VAULT_CONTRACT_ID);
  const usdcContract = new StellarSdk.Contract(USDC_CONTRACT_ID);

  const sourceKeypair = StellarSdk.Keypair.random();
  const sourceAccount = new StellarSdk.Account(sourceKeypair.publicKey(), '0');

  console.log('Checking decimals configuration...\n');

  // Check USDC decimals
  console.log('1. USDC decimals:');
  try {
    const tx1 = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(usdcContract.call('decimals'))
      .setTimeout(180)
      .build();

    const simResult1 = await server.simulateTransaction(tx1);
    if (StellarSdk.rpc.Api.isSimulationSuccess(simResult1)) {
      const decimals = simResult1.result.retval.u32();
      console.log('  USDC decimals:', decimals);
    }
  } catch (e) {
    console.log('  Error:', e.message);
  }

  // Check vault share decimals
  console.log('\n2. Vault share decimals:');
  try {
    const tx2 = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(vaultContract.call('decimals'))
      .setTimeout(180)
      .build();

    const simResult2 = await server.simulateTransaction(tx2);
    if (StellarSdk.rpc.Api.isSimulationSuccess(simResult2)) {
      const decimals = simResult2.result.retval.u32();
      console.log('  Vault decimals:', decimals);
    }
  } catch (e) {
    console.log('  Error:', e.message);
  }

  // Try to read decimals offset from storage
  console.log('\n3. Decimals offset (from storage):');
  try {
    // OpenZeppelin stores this under a specific key - we need to try different keys
    const keys = [
      'DecimalsOffset',
      'DeciOffset',
      'Offset'
    ];

    for (const keyName of keys) {
      try {
        const key = StellarSdk.xdr.ScVal.scvSymbol(keyName);
        const data = await server.getContractData(VAULT_CONTRACT_ID, key);
        if (data && data.val) {
          const scVal = data.val.contractData().val();
          console.log(`  Found ${keyName}:`, scVal.u32());
        }
      } catch (e) {
        // Key not found, try next
      }
    }
  } catch (e) {
    console.log('  Could not read decimals offset');
  }

  // Calculate share price
  console.log('\n4. Share price calculation:');
  const totalAssets = 280182n;
  const totalShares = 20000300000n;
  console.log('  Total assets:', totalAssets.toString(), 'stroops (0.0280182 USDC)');
  console.log('  Total shares:', totalShares.toString(), 'stroops (2000.03 shares)');
  console.log('  Ratio: 1 share =', (Number(totalAssets) / Number(totalShares)).toFixed(10), 'USDC');
  console.log('  Expected: 1 share = 1 USDC (for decimals_offset = 0)');
}

testDecimals();
