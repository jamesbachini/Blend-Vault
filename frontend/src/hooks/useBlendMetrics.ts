import { Backstop, PoolUser, PoolV2 } from '@blend-capital/blend-sdk';
import { useEffect, useState } from 'react';
import {
  BLEND_POOL_CONTRACT_ID,
  NETWORK_PASSPHRASE,
  SOROBAN_RPC_URL,
  USDC_CONTRACT_ID,
  VAULT_CONTRACT_ID,
} from '../utils/stellar';
import { compoundDaily, estimateEmissionsApr } from '../utils/blend';

interface BlendMetricsState {
  apr: number | null;
  baseApr: number | null;
  emissionApr: number | null;
  poolTvl: number | null;
  vaultTvl: number | null;
  isLoading: boolean;
  error?: string;
  lastUpdated?: number;
}

const NETWORK = {
  rpc: SOROBAN_RPC_URL,
  passphrase: NETWORK_PASSPHRASE,
};

let cachedBackstop: { id: string; data: Backstop } | null = null;

async function loadBackstop(backstopId: string) {
  if (!backstopId) {
    return null;
  }

  if (cachedBackstop && cachedBackstop.id === backstopId) {
    return cachedBackstop.data;
  }

  const data = await Backstop.load(NETWORK, backstopId);
  cachedBackstop = { id: backstopId, data };
  return data;
}

async function fetchBlendMetrics() {
  const pool = await PoolV2.load(NETWORK, BLEND_POOL_CONTRACT_ID);
  const reserve = pool.reserves.get(USDC_CONTRACT_ID);

  if (!reserve) {
    throw new Error('USDC reserve not found in Blend pool');
  }

  // Blend UI surfaces the estimated APY (weekly compounding) rather than the raw APR
  const baseAprDecimal = reserve.estSupplyApy ?? reserve.supplyApr ?? 0;

  let emissionAprDecimal = 0;
  if (reserve.supplyEmissions) {
    const emissionsPerAsset = reserve.supplyEmissions.emissionsPerYearPerToken(
      reserve.totalSupply(),
      reserve.config.decimals
    );

    if (emissionsPerAsset > 0) {
      const backstop = await loadBackstop(pool.metadata.backstop);
      if (backstop) {
        emissionAprDecimal = estimateEmissionsApr(
          emissionsPerAsset,
          backstop.backstopToken,
          1 // USDC is already priced in USD
        );
      }
    }
  }

  const compoundedEmissionApr = compoundDaily(emissionAprDecimal);
  const totalAprDecimal = baseAprDecimal + compoundedEmissionApr;

  const poolTvl = reserve.totalSupplyFloat();

  let vaultTvl = 0;
  try {
    const poolUser = await PoolUser.load(
      NETWORK,
      BLEND_POOL_CONTRACT_ID,
      pool,
      VAULT_CONTRACT_ID
    );
    vaultTvl =
      poolUser.getCollateralFloat(reserve) +
      poolUser.getSupplyFloat(reserve);
  } catch (error) {
    console.warn('Failed to load vault TVL from Blend pool', error);
  }

  return {
    apr: totalAprDecimal * 100,
    baseApr: baseAprDecimal * 100,
    emissionApr: compoundedEmissionApr * 100,
    poolTvl,
    vaultTvl,
    lastUpdated: Date.now(),
  };
}

export function useBlendMetrics(refreshMs = 60_000): BlendMetricsState {
  const [state, setState] = useState<BlendMetricsState>({
    apr: null,
    baseApr: null,
    emissionApr: null,
    poolTvl: null,
    vaultTvl: null,
    isLoading: true,
  });

  useEffect(() => {
    let disposed = false;
    let refreshHandle: number | undefined;

    const loadMetrics = async (showSpinner: boolean) => {
      if (showSpinner) {
        setState((prev) => ({ ...prev, isLoading: true, error: undefined }));
      }

      try {
        const metrics = await fetchBlendMetrics();
        if (disposed) return;
        setState({ ...metrics, isLoading: false, error: undefined });
      } catch (error) {
        if (disposed) return;
        setState((prev) => ({
          ...prev,
          isLoading: false,
          error: error instanceof Error ? error.message : 'Failed to load Blend metrics',
        }));
      } finally {
        if (!disposed) {
          refreshHandle = window.setTimeout(() => loadMetrics(false), refreshMs);
        }
      }
    };

    loadMetrics(true);

    return () => {
      disposed = true;
      if (refreshHandle) {
        window.clearTimeout(refreshHandle);
      }
    };
  }, [refreshMs]);

  return state;
}
