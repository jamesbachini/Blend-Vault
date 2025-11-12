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
    const tx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'balance',
      [addressToScVal(userAddress)]
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    return BigInt(0);
  } catch (error) {
    console.error('Error getting share balance:', error);
    return BigInt(0);
  }
}

/**
 * Convert shares to USDC assets using the correct formula
 *
 * Note: The vault's convert_to_assets uses the audited OpenZeppelin implementation,
 * but it calculates based on the vault's local USDC balance (which is 0 since we
 * store everything in Blend). This function uses the correct total_assets from Blend.
 */
export async function convertToAssets(shares: bigint, userAddress: string): Promise<bigint> {
  if (shares === BigInt(0)) {
    return BigInt(0);
  }

  try {
    // Get total supply of shares
    const totalSupplyTx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'total_supply',
      []
    );
    const totalSupplyResult = await sorobanServer.simulateTransaction(totalSupplyTx);

    if (!StellarSdk.rpc.Api.isSimulationSuccess(totalSupplyResult) || !totalSupplyResult.result) {
      throw new Error('Failed to get total supply');
    }
    const totalSupply = scValToNumber(totalSupplyResult.result.retval);

    if (totalSupply === BigInt(0)) {
      return BigInt(0);
    }

    // Get total assets (from Blend pool)
    const totalAssetsTx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'total_assets',
      []
    );
    const totalAssetsResult = await sorobanServer.simulateTransaction(totalAssetsTx);

    if (!StellarSdk.rpc.Api.isSimulationSuccess(totalAssetsResult) || !totalAssetsResult.result) {
      throw new Error('Failed to get total assets');
    }
    const totalAssets = scValToNumber(totalAssetsResult.result.retval);

    // Formula: assets = shares * total_assets / total_supply
    const assets = (shares * totalAssets) / totalSupply;
    return assets;
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
 * Withdraw USDC assets from the vault
 * This is the recommended way to withdraw - just specify the asset amount
 */
export async function withdraw(assets: bigint, userAddress: string): Promise<string> {
  const tx = await buildAndSimulateTransaction(
    userAddress,
    vaultContract,
    'withdraw',
    [
      numberToI128(assets),
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
 * Compound BLND rewards back into the vault
 */
export async function compound(userAddress: string): Promise<string> {
  const tx = await buildAndSimulateTransaction(userAddress, vaultContract, 'compound', [
    addressToScVal(userAddress),
  ]);

  const txXdr = tx.toEnvelope().toXDR('base64');

  const { signedTxXdr } = await StellarWalletsKit.signTransaction(txXdr, {
    networkPassphrase: NETWORK_PASSPHRASE,
    address: userAddress,
  });

  return await submitTransaction(signedTxXdr);
}

/**
 * Redeem shares for USDC (alternative to withdraw)
 * Only use this if you want to redeem a specific number of shares
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
 * Get total supply of vault shares
 */
export async function getTotalSupply(userAddress: string): Promise<bigint> {
  try {
    const tx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'total_supply',
      []
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    return BigInt(0);
  } catch (error) {
    console.error('Error getting total supply:', error);
    return BigInt(0);
  }
}

/**
 * Get total assets in the vault (from Blend)
 */
export async function getTotalAssets(userAddress: string): Promise<bigint> {
  try {
    const tx = await buildAndSimulateTransaction(
      userAddress,
      vaultContract,
      'total_assets',
      []
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.rpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    return BigInt(0);
  } catch (error) {
    console.error('Error getting total assets:', error);
    return BigInt(0);
  }
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
