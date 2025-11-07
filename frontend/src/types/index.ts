export interface WalletState {
  address: string;
  isConnected: boolean;
}

export interface Balances {
  usdc: string;
  vaultUsdc: string;
}

export interface TransactionStatus {
  isLoading: boolean;
  error: string | null;
}
