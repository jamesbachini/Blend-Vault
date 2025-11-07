import * as StellarSdk from '@stellar/stellar-sdk';
import { StellarWalletsKit } from '@creit-tech/stellar-wallets-kit/sdk';
import {
  USDC_CONTRACT_ID,
  VAULT_CONTRACT_ID,
  addressToScVal,
  numberToI128,
  scValToNumber,
  buildAndSimulateTransaction,
  submitTransaction,
  NETWORK_PASSPHRASE,
  sorobanServer,
} from '../utils/stellar';

const usdcContract = new StellarSdk.Contract(USDC_CONTRACT_ID);

/**
 * Get USDC balance for an address
 */
export async function getBalance(address: string): Promise<bigint> {
  try {
    const tx = await buildAndSimulateTransaction(
      address,
      usdcContract,
      'balance',
      [addressToScVal(address)]
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.SorobanRpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    return BigInt(0);
  } catch (error) {
    console.error('Error getting USDC balance:', error);
    return BigInt(0);
  }
}

/**
 * Get USDC allowance for vault contract
 */
export async function getAllowance(ownerAddress: string): Promise<bigint> {
  try {
    const tx = await buildAndSimulateTransaction(
      ownerAddress,
      usdcContract,
      'allowance',
      [addressToScVal(ownerAddress), addressToScVal(VAULT_CONTRACT_ID)]
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.SorobanRpc.Api.isSimulationSuccess(result) && result.result) {
      return scValToNumber(result.result.retval);
    }

    return BigInt(0);
  } catch (error) {
    console.error('Error getting allowance:', error);
    return BigInt(0);
  }
}

/**
 * Approve vault contract to spend USDC
 */
export async function approve(amount: bigint, ownerAddress: string): Promise<string> {
  if (!ownerAddress || ownerAddress.trim() === '') {
    throw new Error('Invalid owner address: address is empty');
  }

  // Set expiration ledger (approximately 30 days from now at ~5 seconds per ledger)
  const currentLedger = (await sorobanServer.getLatestLedger()).sequence;
  const expirationLedger = currentLedger + 518400; // ~30 days

  const tx = await buildAndSimulateTransaction(
    ownerAddress,
    usdcContract,
    'approve',
    [
      addressToScVal(ownerAddress),
      addressToScVal(VAULT_CONTRACT_ID),
      numberToI128(amount),
      StellarSdk.nativeToScVal(expirationLedger, { type: 'u32' }),
    ]
  );

  // Convert to base64 XDR explicitly
  const txXdr = tx.toEnvelope().toXDR('base64');

  const signResult = await StellarWalletsKit.signTransaction(txXdr, {
    networkPassphrase: NETWORK_PASSPHRASE,
    address: ownerAddress,
  });

  const signedTxXdr = signResult.signedTxXdr || signResult;

  return await submitTransaction(signedTxXdr);
}

/**
 * Get token decimals (should be 7 for USDC)
 */
export async function getDecimals(userAddress: string): Promise<number> {
  try {
    const tx = await buildAndSimulateTransaction(
      userAddress,
      usdcContract,
      'decimals',
      []
    );

    const result = await sorobanServer.simulateTransaction(tx);

    if (StellarSdk.SorobanRpc.Api.isSimulationSuccess(result) && result.result) {
      return Number(scValToNumber(result.result.retval));
    }

    return 7; // Default to 7 for USDC
  } catch (error) {
    console.error('Error getting decimals:', error);
    return 7;
  }
}
