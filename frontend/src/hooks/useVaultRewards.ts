import { PoolUser, PoolV2 } from '@blend-capital/blend-sdk';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  BLEND_POOL_CONTRACT_ID,
  NETWORK_PASSPHRASE,
  SOROBAN_RPC_URL,
  USDC_CONTRACT_ID,
  VAULT_CONTRACT_ID,
} from '../utils/stellar';

const NETWORK = {
  rpc: SOROBAN_RPC_URL,
  passphrase: NETWORK_PASSPHRASE,
};

interface UseVaultRewardsOptions {
  refreshMs?: number;
  enabled?: boolean;
}

interface UseVaultRewardsState {
  pendingBlnd: number | null;
  isLoading: boolean;
  error?: string;
}

export interface UseVaultRewardsResult extends UseVaultRewardsState {
  refresh: (withSpinner?: boolean) => Promise<void>;
}

export function useVaultRewards({
  refreshMs = 60_000,
  enabled = true,
}: UseVaultRewardsOptions = {}): UseVaultRewardsResult {
  const [state, setState] = useState<UseVaultRewardsState>({
    pendingBlnd: null,
    isLoading: enabled,
    error: undefined,
  });

  const isMountedRef = useRef(true);

  useEffect(() => {
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const refresh = useCallback(
    async (withSpinner = true) => {
      if (!enabled) {
        if (isMountedRef.current) {
          setState({ pendingBlnd: null, isLoading: false, error: undefined });
        }
        return;
      }

      if (withSpinner && isMountedRef.current) {
        setState((prev) => ({ ...prev, isLoading: true, error: undefined }));
      }

      try {
        const pool = await PoolV2.load(NETWORK, BLEND_POOL_CONTRACT_ID);
        const usdcReserve = pool.reserves.get(USDC_CONTRACT_ID);

        if (!usdcReserve) {
          throw new Error('USDC reserve not found in Blend pool');
        }

        const poolUser = await PoolUser.load(
          NETWORK,
          BLEND_POOL_CONTRACT_ID,
          pool,
          VAULT_CONTRACT_ID
        );

        const reservesForEmission = usdcReserve ? [usdcReserve] : Array.from(pool.reserves.values());
        const { emissions } = poolUser.estimateEmissions(reservesForEmission);

        if (import.meta.env.DEV) {
          console.debug('[BlendVault] Pending BLND rewards', emissions);
        }

        if (!isMountedRef.current) {
          return;
        }

        setState({
          pendingBlnd: emissions,
          isLoading: false,
          error: undefined,
        });
      } catch (error) {
        console.error('Failed to load BLND rewards', error);
        if (!isMountedRef.current) {
          return;
        }

        const errorMessage =
          error instanceof Error ? error.message : 'Failed to load BLND rewards';
        const isMissingPosition =
          errorMessage.includes('Unable to load user') ||
          errorMessage.includes('Unable to load reserve') ||
          errorMessage.includes('missing ledger entries');

        if (isMissingPosition) {
          setState({
            pendingBlnd: 0,
            isLoading: false,
            error: undefined,
          });
        } else {
          setState((prev) => ({
            ...prev,
            isLoading: false,
            error: errorMessage,
          }));
        }
      }
    },
    [enabled]
  );

  useEffect(() => {
    if (!enabled) {
      setState({ pendingBlnd: null, isLoading: false, error: undefined });
      return;
    }

    let intervalId: number | undefined;

    refresh();

    intervalId = window.setInterval(() => {
      refresh(false);
    }, refreshMs);

    return () => {
      if (intervalId) {
        window.clearInterval(intervalId);
      }
    };
  }, [enabled, refresh, refreshMs]);

  return {
    ...state,
    refresh,
  };
}
