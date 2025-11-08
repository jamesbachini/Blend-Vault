#!/usr/bin/env node

const StellarSdk = require('@stellar/stellar-sdk');

const VAULT_CONTRACT_ID = 'CDGSSWMTWASCHMRPPWZ2YQJOSARYJRIFJDNNA2COB35BXACESZE7RXHQ';
const BLEND_POOL_ID = 'CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS';
const SOROBAN_RPC_URL = 'https://rpc.lightsail.network/';
const NETWORK_PASSPHRASE = StellarSdk.Networks.PUBLIC;

async function testBlendPosition() {
  const server = new StellarSdk.rpc.Server(SOROBAN_RPC_URL);
  const blendContract = new StellarSdk.Contract(BLEND_POOL_ID);
  const vaultAddress = StellarSdk.nativeToScVal(VAULT_CONTRACT_ID, { type: 'address' });

  const sourceKeypair = StellarSdk.Keypair.random();
  const sourceAccount = new StellarSdk.Account(sourceKeypair.publicKey(), '0');

  console.log('Querying Blend pool positions for vault...\n');
  console.log('Vault:', VAULT_CONTRACT_ID);
  console.log('Blend Pool:', BLEND_POOL_ID);
  console.log('---\n');

  try {
    const tx = new StellarSdk.TransactionBuilder(sourceAccount, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(blendContract.call('get_positions', vaultAddress))
      .setTimeout(180)
      .build();

    const simResult = await server.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(simResult)) {
      const result = simResult.result.retval;

      console.log('Vault positions in Blend pool:');
      console.log('---\n');

      // Blend returns Positions as a Map with symbol keys
      const positionsMap = result.map();

      // Helper to find a field in the map
      const findField = (fieldName) => {
        for (const entry of positionsMap) {
          const keyBytes = entry.key().sym();
          const keyStr = Buffer.from(keyBytes).toString();
          if (keyStr === fieldName) {
            return entry.val().map() || [];
          }
        }
        return [];
      };

      // Extract collateral map
      const collateralMap = findField('collateral');
      console.log('Collateral:');
      if (collateralMap.length === 0) {
        console.log('  (empty)');
      } else {
        for (const entry of collateralMap) {
          const key = entry.key().u32();
          const val = StellarSdk.scValToBigInt(entry.val());
          console.log(`  Index ${key}: ${val.toString()} stroops (${(Number(val) / 1e7).toFixed(7)} tokens)`);
        }
      }

      // Extract liabilities map
      const liabilitiesMap = findField('liabilities');
      console.log('\nLiabilities:');
      if (liabilitiesMap.length === 0) {
        console.log('  (empty)');
      } else {
        for (const entry of liabilitiesMap) {
          const key = entry.key().u32();
          const val = StellarSdk.scValToBigInt(entry.val());
          console.log(`  Index ${key}: ${val.toString()} stroops (${(Number(val) / 1e7).toFixed(7)} tokens)`);
        }
      }

      // Extract supply map
      const supplyMap = findField('supply');
      console.log('\nSupply:');
      if (supplyMap.length === 0) {
        console.log('  (empty - NO USDC IN BLEND!)');
      } else {
        for (const entry of supplyMap) {
          const key = entry.key().u32();
          const val = StellarSdk.scValToBigInt(entry.val());
          console.log(`  Index ${key}: ${val.toString()} stroops (${(Number(val) / 1e7).toFixed(7)} tokens)`);
        }
      }

      console.log('\n---');
      console.log('Vault config expects USDC at index: 1');
      console.log('\nIf index 1 is empty or doesn\'t match 0.03 USDC, the reserve index is WRONG.');

    } else {
      console.log('Simulation error:', simResult.error);
    }
  } catch (error) {
    console.error('Error:', error.message);
  }
}

testBlendPosition();
