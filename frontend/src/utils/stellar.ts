import * as StellarSdk from '@stellar/stellar-sdk';

// Mainnet configuration
export const NETWORK_PASSPHRASE = StellarSdk.Networks.PUBLIC;
export const HORIZON_URL = 'https://horizon.stellar.org';
export const SOROBAN_RPC_URL = 'https://rpc.lightsail.network/';

// Contract addresses
export const VAULT_CONTRACT_ID = 'CCZWCNTCTHO3FE6YCYX6YYWFR3B3BEVICD42RZZFMWSPDEIFPQYW4IHA';
export const USDC_CONTRACT_ID = 'CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75'; // Stellar USDC on mainnet

// Initialize Soroban server
export const sorobanServer = new StellarSdk.SorobanRpc.Server(SOROBAN_RPC_URL);

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

  if (StellarSdk.SorobanRpc.Api.isSimulationError(simulatedTransaction)) {
    throw new Error(`Simulation failed: ${simulatedTransaction.error}`);
  }

  const preparedTransaction = StellarSdk.SorobanRpc.assembleTransaction(
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
  let response = sendResponse;
  while (response.status === 'PENDING' || response.status === 'NOT_FOUND') {
    await new Promise(resolve => setTimeout(resolve, 1000));
    response = await sorobanServer.getTransaction(txHash);
  }

  if (response.status === 'SUCCESS') {
    return txHash;
  } else {
    // Extract detailed error information
    let errorMessage = `Transaction failed: ${response.status}`;

    if (StellarSdk.SorobanRpc.Api.isSimulationError(response)) {
      errorMessage += ` - ${response.error}`;
    }

    // Check for result codes in the response
    if ('resultXdr' in response && response.resultXdr) {
      try {
        const result = StellarSdk.xdr.TransactionResult.fromXDR(response.resultXdr, 'base64');
        console.error('Transaction result XDR:', result);
        errorMessage += ` - Result: ${JSON.stringify(result)}`;
      } catch (e) {
        console.error('Could not parse result XDR:', e);
      }
    }

    console.error('Full error response:', JSON.stringify(response, null, 2));
    throw new Error(errorMessage);
  }
}
