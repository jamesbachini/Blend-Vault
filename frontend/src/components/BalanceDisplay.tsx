import React from 'react';
import { formatUSDCWithCommas } from '../utils/format';
import './BalanceDisplay.css';

interface BalanceDisplayProps {
  walletBalance: bigint | null;
  vaultBalance: bigint | null;
  isLoading: boolean;
}

const SkeletonLoader: React.FC = () => (
  <div className="skeleton-loader">
    <div className="skeleton-shimmer" />
  </div>
);

export const BalanceDisplay: React.FC<BalanceDisplayProps> = ({
  walletBalance,
  vaultBalance,
  isLoading,
}) => {
  return (
    <div className="balance-display">
      <div className="balance-card">
        <div className="balance-header">
          <div className="balance-icon balance-icon--wallet">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path
                d="M2 5a2 2 0 012-2h12a2 2 0 012 2v10a2 2 0 01-2 2H4a2 2 0 01-2-2V5z"
                stroke="currentColor"
                strokeWidth="2"
                fill="none"
              />
              <circle cx="13" cy="10" r="1.5" fill="currentColor" />
            </svg>
          </div>
          <span className="balance-label">Wallet Balance</span>
        </div>
        <div className="balance-value">
          {isLoading ? (
            <SkeletonLoader />
          ) : walletBalance !== null ? (
            <>
              <span className="balance-amount">{formatUSDCWithCommas(walletBalance)}</span>
              <span className="balance-currency">USDC</span>
            </>
          ) : (
            <span className="balance-empty">--</span>
          )}
        </div>
      </div>

      <div className="balance-card">
        <div className="balance-header">
          <div className="balance-icon balance-icon--vault">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <rect
                x="3"
                y="3"
                width="14"
                height="14"
                rx="2"
                stroke="currentColor"
                strokeWidth="2"
                fill="none"
              />
              <circle cx="10" cy="10" r="3" stroke="currentColor" strokeWidth="2" fill="none" />
              <circle cx="10" cy="10" r="1" fill="currentColor" />
            </svg>
          </div>
          <span className="balance-label">Vault Balance</span>
        </div>
        <div className="balance-value">
          {isLoading ? (
            <SkeletonLoader />
          ) : vaultBalance !== null ? (
            <>
              <span className="balance-amount">{formatUSDCWithCommas(vaultBalance)}</span>
              <span className="balance-currency">USDC</span>
            </>
          ) : (
            <span className="balance-empty">--</span>
          )}
        </div>
        {vaultBalance !== null && vaultBalance > BigInt(0) && (
          <div className="balance-hint">Earning yield from Blend Protocol</div>
        )}
      </div>
    </div>
  );
};
