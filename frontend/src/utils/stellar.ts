import * as StellarSdk from '@stellar/stellar-sdk';

// Mainnet configuration
export const NETWORK_PASSPHRASE = StellarSdk.Networks.PUBLIC;
export const HORIZON_URL = 'https://horizon.stellar.org';
export const SOROBAN_RPC_URL = 'https://rpc.lightsail.network/';

// Contract addresses
export const VAULT_CONTRACT_ID = 'CDGSSWMTWASCHMRPPWZ2YQJOSARYJRIFJDNNA2COB35BXACESZE7RXHQ';
export const USDC_CONTRACT_ID = 'CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75'; // Stellar USDC on mainnet

// Initialize Soroban server
export const sorobanServer = new StellarSdk.rpc.Server(SOROBAN_RPC_URL);

// Initialize Horizon server
export const horizonServer = new StellarSdk.Horizon.Server(HORIZON_URL);

// Convert address to ScVal
export function addressToScVal(address: string): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(address, { type: 'address' });
}

// Convert number to ScVal (i128)
export function numberToI128(value: bigint): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(value, { type: 'i128' });
}

// Convert ScVal to number
export function scValToNumber(scVal: StellarSdk.xdr.ScVal): bigint {
  return StellarSdk.scValToBigInt(scVal);
}

// Build and simulate transaction
export async function buildAndSimulateTransaction(
  sourceAddress: string,
  contract: StellarSdk.Contract,
  method: string,
  params: StellarSdk.xdr.ScVal[]
): Promise<StellarSdk.Transaction> {
  const sourceAccount = await horizonServer.loadAccount(sourceAddress);

  const transaction = new StellarSdk.TransactionBuilder(sourceAccount, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...params))
    .setTimeout(180)
    .build();

  const simulatedTransaction = await sorobanServer.simulateTransaction(transaction);

  if (StellarSdk.rpc.Api.isSimulationError(simulatedTransaction)) {
    throw new Error(`Simulation failed: ${simulatedTransaction.error}`);
  }

  const preparedTransaction = StellarSdk.rpc.assembleTransaction(
    transaction,
    simulatedTransaction
  ).build();

  return preparedTransaction;
}

// Submit transaction after signing
export async function submitTransaction(signedXdr: string): Promise<string> {
  const transaction = StellarSdk.TransactionBuilder.fromXDR(signedXdr, NETWORK_PASSPHRASE);

  let sendResponse = await sorobanServer.sendTransaction(transaction);

  // Store the hash separately since it might not be in the getTransaction response
  const txHash = sendResponse.hash;

  // Poll for transaction status
  let getResponse = await sorobanServer.getTransaction(txHash);
  while (getResponse.status === StellarSdk.rpc.Api.GetTransactionStatus.NOT_FOUND) {
    await new Promise(resolve => setTimeout(resolve, 1000));
    getResponse = await sorobanServer.getTransaction(txHash);
  }

  if (getResponse.status === StellarSdk.rpc.Api.GetTransactionStatus.SUCCESS) {
    return txHash;
  } else {
    // Extract detailed error information
    let errorMessage = `Transaction failed: ${getResponse.status}`;
    console.error('Full error response:', JSON.stringify(getResponse, null, 2));
    throw new Error(errorMessage);
  }
}
