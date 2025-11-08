#!/usr/bin/env node

const StellarSdk = require('@stellar/stellar-sdk');

const VAULT_CONTRACT_ID = 'CDGSSWMTWASCHMRPPWZ2YQJOSARYJRIFJDNNA2COB35BXACESZE7RXHQ';
const SOROBAN_RPC_URL = 'https://rpc.lightsail.network/';
const NETWORK_PASSPHRASE = StellarSdk.Networks.PUBLIC;

async function diagnoseShares() {
  const server = new StellarSdk.rpc.Server(SOROBAN_RPC_URL);
  const vaultContract = new StellarSdk.Contract(VAULT_CONTRACT_ID);

  const sourceKeypair = StellarSdk.Keypair.random();
  const sourceAccount = new StellarSdk.Account(sourceKeypair.publicKey(), '0');

  console.log('Diagnosing share calculation issue...\n');

  // Get current state
  const totalAssets = 280182n;
  const totalSupply = 20000300000n;
  const testDeposit = 1000000n; // 0.1 USDC

  console.log('Current vault state:');
  console.log('  total_assets:', totalAssets.toString(), 'stroops');
  console.log('  total_supply:', totalSupply.toString(), 'shares');
  console.log('');

  // Test preview_deposit
  console.log('Testing preview_deposit(1000000):');
  try {
    const tx = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(vaultContract.call('preview_deposit', StellarSdk.nativeToScVal(testDeposit, { type: 'i128' })))
      .setTimeout(180)
      .build();

    const result = await server.simulateTransaction(tx);
    if (StellarSdk.rpc.Api.isSimulationSuccess(result)) {
      const shares = StellarSdk.scValToBigInt(result.result.retval);
      console.log('  Returns:', shares.toString(), 'shares');
      console.log('');

      // Calculate expected with ERC-4626 formula
      const expectedShares = (testDeposit * totalSupply) / totalAssets;
      console.log('Expected (manual calculation):');
      console.log('  Formula: shares = assets * total_supply / total_assets');
      console.log(`  shares = ${testDeposit} * ${totalSupply} / ${totalAssets}`);
      console.log(`  shares = ${expectedShares}`);
      console.log('');

      const ratio = Number(shares) / Number(expectedShares);
      console.log(`Actual / Expected ratio: ${ratio.toFixed(2)}x`);

      if (ratio !== 1.0) {
        console.log('');
        console.log('❌ CALCULATION IS WRONG!');
        console.log('');
        console.log('The vault is using a different formula than standard ERC-4626.');
        console.log('This suggests either:');
        console.log('  1. Wrong total_assets() being returned');
        console.log('  2. Wrong total_supply() being returned');
        console.log('  3. Bug in OpenZeppelin Vault implementation');
        console.log('  4. Decimals_offset is not actually 0');
      }
    }
  } catch (e) {
    console.log('Error:', e.message);
  }

  // Test convert_to_shares
  console.log('\n---\nTesting convert_to_shares(1000000):');
  try {
    const tx = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(vaultContract.call('convert_to_shares', StellarSdk.nativeToScVal(testDeposit, { type: 'i128' })))
      .setTimeout(180)
      .build();

    const result = await server.simulateTransaction(tx);
    if (StellarSdk.rpc.Api.isSimulationSuccess(result)) {
      const shares = StellarSdk.scValToBigInt(result.result.retval);
      console.log('  Returns:', shares.toString(), 'shares');
    }
  } catch (e) {
    console.log('Error:', e.message);
  }

  // Test convert_to_assets going backwards
  console.log('\n---\nTesting convert_to_assets(71429095500):');
  const testShares = 71429095500n; // What 1000000 assets should give us
  try {
    const tx = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(vaultContract.call('convert_to_assets', StellarSdk.nativeToScVal(testShares, { type: 'i128' })))
      .setTimeout(180)
      .build();

    const result = await server.simulateTransaction(tx);
    if (StellarSdk.rpc.Api.isSimulationSuccess(result)) {
      const assets = StellarSdk.scValToBigInt(result.result.retval);
      console.log('  Returns:', assets.toString(), 'stroops');
      console.log('  That\'s:', (Number(assets) / 1e7).toFixed(7), 'USDC');
    }
  } catch (e) {
    console.log('Error:', e.message);
  }
}

diagnoseShares();
