#!/usr/bin/env node

const StellarSdk = require('@stellar/stellar-sdk');

const VAULT_CONTRACT_ID = 'CDGSSWMTWASCHMRPPWZ2YQJOSARYJRIFJDNNA2COB35BXACESZE7RXHQ';
const SOROBAN_RPC_URL = 'https://rpc.lightsail.network/';
const NETWORK_PASSPHRASE = StellarSdk.Networks.PUBLIC;

async function checkInitState() {
  const server = new StellarSdk.rpc.Server(SOROBAN_RPC_URL);
  const vaultContract = new StellarSdk.Contract(VAULT_CONTRACT_ID);

  const sourceKeypair = StellarSdk.Keypair.random();
  const sourceAccount = new StellarSdk.Account(sourceKeypair.publicKey(), '0');

  console.log('Checking vault initialization state...\n');

  // Get all ledger entries for this contract to see what's stored
  try {
    const ledgerKey = StellarSdk.xdr.LedgerKey.contractData(
      new StellarSdk.xdr.LedgerKeyContractData({
        contract: new StellarSdk.Address(VAULT_CONTRACT_ID).toScAddress(),
        key: StellarSdk.xdr.ScVal.scvLedgerKeyContractInstance(),
        durability: StellarSdk.xdr.ContractDataDurability.persistent(),
      })
    );

    const response = await server.getLedgerEntries(ledgerKey);
    console.log('Contract instance data:', JSON.stringify(response, null, 2));
  } catch (e) {
    console.log('Error reading instance:', e.message);
  }

  // Try to calculate what the first deposit would get
  console.log('\n---\nSimulating first deposit calculation:\n');

  // When vault is empty: shares = assets * 10^decimals_offset
  // decimals_offset = 0, so shares should = assets
  console.log('Formula when total_assets = 0 and total_supply = 0:');
  console.log('  shares = assets * 10^decimals_offset');
  console.log('  shares = assets * 10^0');
  console.log('  shares = assets * 1');
  console.log('  shares = assets');
  console.log('');
  console.log('Expected for 0.01 USDC (100,000 stroops):');
  console.log('  Should get: 100,000 shares');
  console.log('  Actually got: ~10,000,050,000 shares (100,000x too many!)');
  console.log('');
  console.log('Expected for 0.02 USDC (200,000 stroops):');
  console.log('  Should get: 200,000 shares');
  console.log('  Actually got: ~10,000,250,000 shares (50,000x too many!)');
}

checkInitState();
