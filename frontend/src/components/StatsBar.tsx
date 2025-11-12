import React from 'react';
import { useBlendMetrics } from '../hooks/useBlendMetrics';
import { formatPercentage, formatUsd } from '../utils/format';
import './StatsBar.css';

const StatsValue: React.FC<{ label: string; value: string; description?: string; loading: boolean }>
  = ({ label, value, description, loading }) => (
    <div className="stats-item">
      <span className="stats-label">{label}</span>
      <div className="stats-value">
        {loading ? <span className="stats-skeleton" /> : value}
      </div>
      {description && <span className="stats-description">{description}</span>}
    </div>
  );

export const StatsBar: React.FC = () => {
  const { apr, baseApr, emissionApr, poolTvl, vaultTvl, isLoading, error, lastUpdated } =
    useBlendMetrics();

  const showSkeleton = isLoading && apr === null;

  const aprValue = formatPercentage(apr);
  const baseAprValue = formatPercentage(baseApr);
  const emissionAprValue = formatPercentage(emissionApr);
  const aprDescription =
    baseApr !== null && emissionApr !== null
      ? `Base ${baseAprValue} · BLND ${emissionAprValue}`
      : 'Blend supply APY plus auto-compounded BLND incentives';

  const vaultValue = formatUsd(vaultTvl, { maximumFractionDigits: 2 });
  const poolValue = formatUsd(poolTvl, {
    compact: poolTvl !== null && poolTvl >= 1_000_000,
    maximumFractionDigits: 2,
  });

  const infoLine = error
    ? `Data refresh failed: ${error}`
    : lastUpdated
    ? `Updated ${new Date(lastUpdated).toLocaleTimeString()}`
    : '';

  return (
    <section className="stats-bar">
      <div className="stats-bar-container">
        <div className="stats-items">
          <StatsValue
            label="APY"
            value={aprValue}
            description={aprDescription}
            loading={showSkeleton}
          />
          <div className="stats-divider" aria-hidden="true" />
          <StatsValue
            label="Vault TVL"
            value={vaultValue}
            description="USDC Vault Deposits"
            loading={showSkeleton}
          />
          <div className="stats-divider" aria-hidden="true" />
          <StatsValue
            label="Pool TVL"
            value={poolValue}
            description="USDC in Blend Pool"
            loading={showSkeleton}
          />
        </div>
        {infoLine && <div className={`stats-updated ${error ? 'stats-error' : ''}`}>{infoLine}</div>}
      </div>
    </section>
  );
};
