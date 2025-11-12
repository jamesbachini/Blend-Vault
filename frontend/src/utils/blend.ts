import { BackstopToken, FixedMath } from '@blend-capital/blend-sdk';

/**
 * Estimate the emissions APR (in decimal form) for a reserve.
 */
export function estimateEmissionsApr(
  emissionsPerAssetPerYear: number,
  backstopToken: BackstopToken,
  assetPriceUsd: number
): number {
  if (!assetPriceUsd || assetPriceUsd <= 0) {
    return 0;
  }

  const usdcPerBlnd =
    FixedMath.toFloat(backstopToken.usdc, 7) /
    0.2 /
    (FixedMath.toFloat(backstopToken.blnd, 7) / 0.8);

  return (emissionsPerAssetPerYear * usdcPerBlnd) / assetPriceUsd;
}

/**
 * Convert a simple APR (decimal) to a daily-compounded APR approximation.
 */
export function compoundDaily(aprDecimal: number): number {
  if (!aprDecimal || aprDecimal <= 0) {
    return 0;
  }

  return Math.pow(1 + aprDecimal / 365, 365) - 1;
}
