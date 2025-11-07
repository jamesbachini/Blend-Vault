import React, { useEffect, useRef } from 'react';
import { StellarWalletsKit } from '@creit-tech/stellar-wallets-kit/sdk';
import { formatAddress } from '../utils/format';
import './ConnectButton.css';

interface ConnectButtonProps {
  address: string;
  isConnected: boolean;
}

export const ConnectButton: React.FC<ConnectButtonProps> = ({ address, isConnected }) => {
  const buttonWrapperRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const buttonWrapper = buttonWrapperRef.current;
    if (buttonWrapper && buttonWrapper.children.length === 0) {
      StellarWalletsKit.createButton(buttonWrapper);
    }
  }, []);

  return (
    <div className="connect-button-container">
      {isConnected && (
        <div className="wallet-info">
          <div className="wallet-icon">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="2" />
              <circle cx="8" cy="8" r="3" fill="currentColor" />
            </svg>
          </div>
          <span className="wallet-address">{formatAddress(address)}</span>
        </div>
      )}
      <div ref={buttonWrapperRef} className="wallet-kit-button" />
    </div>
  );
};
