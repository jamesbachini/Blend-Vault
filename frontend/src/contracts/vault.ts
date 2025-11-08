import * as StellarSdk from '@stellar/stellar-sdk';
import { StellarWalletsKit } from '@creit-tech/stellar-wallets-kit/sdk';
import {
  VAULT_CONTRACT_ID,
  addressToScVal,
  numberToI128,
  scValToNumber,
  buildAndSimulateTransaction,
  submitTransaction,
  NETWORK_PASSPHRASE,
  sorobanServer,
} from '../utils/stellar';

const vaultContract = new StellarSdk.Contract(VAULT_CONTRACT_ID);

/**
 * Get the user's share balance in the vault
 */
export async function getShareBalance(userAddress: string): Promise<bigint> {
  try {
    const result = await sorobanServer.getContractData(
      VAULT_CONTRACT_ID,
      StellarSdk.xdr.ScVal.scvVec([
        StellarSdk.xdr.ScVal.scvSymbol('Balance'),
        addressToScVal(userAddress),
      ])
    );

    if (result && result.val) {
      // In v14, result.val is LedgerEntryData, need to extract ScVal from it
      const ledgerData = result.val;
      const scVal = ledgerData.contractData().val();
      return scValToNumber(scVal);
    }
    return BigInt(0);
  } catch (error) {
    // If no balance exists, return 0
    return BigInt(0);
  }
}

/**
 * Convert shares to USDC assets
 */
export async function convertToAssets(shares: bigint, userAddress: string): Promise<bigint> {
  if (shares === BigInt(0)) {
    return BigInt(0);
  }

  try {
    const tx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'convert_to_assets',
      [numberToI128(shares)]
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    throw new Error('Failed to convert shares to assets');
  } catch (error) {
    console.error('Error converting to assets:', error);
    return BigInt(0);
  }
}

/**
 * Convert USDC assets to shares
 */
export async function convertToShares(assets: bigint, userAddress: string): Promise<bigint> {
  if (assets === BigInt(0)) {
    return BigInt(0);
  }

  try {
    const tx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'convert_to_shares',
      [numberToI128(assets)]
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    throw new Error('Failed to convert assets to shares');
  } catch (error) {
    console.error('Error converting to shares:', error);
    throw error;
  }
}

/**
 * Deposit USDC into the vault
 */
export async function deposit(amount: bigint, userAddress: string): Promise<string> {
  const tx = await buildAndSimulateTransaction(
    userAddress,
    vaultContract,
    'deposit',
    [
      numberToI128(amount),
      addressToScVal(userAddress), // receiver
      addressToScVal(userAddress), // from
      addressToScVal(userAddress), // operator
    ]
  );

  const txXdr = tx.toEnvelope().toXDR('base64');

  const { signedTxXdr } = await StellarWalletsKit.signTransaction(txXdr, {
    networkPassphrase: NETWORK_PASSPHRASE,
    address: userAddress,
  });

  return await submitTransaction(signedTxXdr);
}

/**
 * Redeem shares for USDC (withdraw)
 */
export async function redeem(shares: bigint, userAddress: string): Promise<string> {
  const tx = await buildAndSimulateTransaction(
    userAddress,
    vaultContract,
    'redeem',
    [
      numberToI128(shares),
      addressToScVal(userAddress), // receiver
      addressToScVal(userAddress), // owner
      addressToScVal(userAddress), // operator
    ]
  );

  const txXdr = tx.toEnvelope().toXDR('base64');

  const { signedTxXdr } = await StellarWalletsKit.signTransaction(txXdr, {
    networkPassphrase: NETWORK_PASSPHRASE,
    address: userAddress,
  });

  return await submitTransaction(signedTxXdr);
}

/**
 * Get max deposit amount (for validation)
 */
export async function maxDeposit(userAddress: string): Promise<bigint> {
  try {
    const tx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'max_deposit',
      [addressToScVal(userAddress)]
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    return BigInt(Number.MAX_SAFE_INTEGER);
  } catch (error) {
    console.error('Error getting max deposit:', error);
    return BigInt(Number.MAX_SAFE_INTEGER);
  }
}

/**
 * Get max redeem amount (max shares user can redeem)
 */
export async function maxRedeem(userAddress: string): Promise<bigint> {
  try {
    const tx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'max_redeem',
      [addressToScVal(userAddress)]
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    return BigInt(0);
  } catch (error) {
    console.error('Error getting max redeem:', error);
    return BigInt(0);
  }
}
