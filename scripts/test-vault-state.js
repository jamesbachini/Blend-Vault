#!/usr/bin/env node

const StellarSdk = require('@stellar/stellar-sdk');

const VAULT_CONTRACT_ID = 'CDGSSWMTWASCHMRPPWZ2YQJOSARYJRIFJDNNA2COB35BXACESZE7RXHQ';
const SOROBAN_RPC_URL = 'https://rpc.lightsail.network/';
const NETWORK_PASSPHRASE = StellarSdk.Networks.PUBLIC;

async function testVaultState() {
  const server = new StellarSdk.rpc.Server(SOROBAN_RPC_URL);
  const vaultContract = new StellarSdk.Contract(VAULT_CONTRACT_ID);

  // Dummy source account for simulation
  const sourceKeypair = StellarSdk.Keypair.random();
  const sourceAccount = new StellarSdk.Account(sourceKeypair.publicKey(), '0');

  console.log('Testing vault state...\n');

  // Test total_assets
  console.log('1. Calling total_assets():');
  try {
    const tx1 = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(vaultContract.call('total_assets'))
      .setTimeout(180)
      .build();

    const simResult1 = await server.simulateTransaction(tx1);

    if (StellarSdk.rpc.Api.isSimulationSuccess(simResult1)) {
      const totalAssets = StellarSdk.scValToBigInt(simResult1.result.retval);
      console.log('  Total assets:', totalAssets.toString(), 'stroops');
      console.log('  Total assets:', (Number(totalAssets) / 1e7).toFixed(7), 'USDC');
    } else {
      console.log('  Error:', simResult1.error);
    }
  } catch (e) {
    console.log('  Error:', e.message);
  }

  // Test total_supply
  console.log('\n2. Calling total_supply():');
  try {
    const tx2 = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(vaultContract.call('total_supply'))
      .setTimeout(180)
      .build();

    const simResult2 = await server.simulateTransaction(tx2);

    if (StellarSdk.rpc.Api.isSimulationSuccess(simResult2)) {
      const totalSupply = StellarSdk.scValToBigInt(simResult2.result.retval);
      console.log('  Total supply:', totalSupply.toString(), 'shares');
      console.log('  Total supply:', (Number(totalSupply) / 1e7).toFixed(7), 'shares');
    } else {
      console.log('  Error:', simResult2.error);
    }
  } catch (e) {
    console.log('  Error:', e.message);
  }

  // Test query_asset
  console.log('\n3. Calling query_asset():');
  try {
    const tx3 = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(vaultContract.call('query_asset'))
      .setTimeout(180)
      .build();

    const simResult3 = await server.simulateTransaction(tx3);

    if (StellarSdk.rpc.Api.isSimulationSuccess(simResult3)) {
      const asset = StellarSdk.StrKey.encodeContract(simResult3.result.retval.address().contractId());
      console.log('  Asset:', asset);
    } else {
      console.log('  Error:', simResult3.error);
    }
  } catch (e) {
    console.log('  Error:', e.message);
  }

  // Read vault configuration from storage
  console.log('\n4. Reading vault configuration:');
  try {
    const blendPoolKey = StellarSdk.xdr.ScVal.scvSymbol('BlendPool');
    const blendPoolData = await server.getContractData(VAULT_CONTRACT_ID, blendPoolKey);
    if (blendPoolData && blendPoolData.val) {
      const scVal = blendPoolData.val.contractData().val();
      const blendPool = StellarSdk.StrKey.encodeContract(scVal.address().contractId());
      console.log('  Blend pool:', blendPool);
    }
  } catch (e) {
    console.log('  Blend pool: Error reading -', e.message);
  }

  try {
    const usdcIndexKey = StellarSdk.xdr.ScVal.scvSymbol('USDCReserveIndex');
    const usdcIndexData = await server.getContractData(VAULT_CONTRACT_ID, usdcIndexKey);
    if (usdcIndexData && usdcIndexData.val) {
      const scVal = usdcIndexData.val.contractData().val();
      const usdcIndex = scVal.u32();
      console.log('  USDC reserve index:', usdcIndex);
    }
  } catch (e) {
    console.log('  USDC reserve index: Error reading -', e.message);
  }
}

testVaultState();
