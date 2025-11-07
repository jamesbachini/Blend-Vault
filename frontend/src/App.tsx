import React, { useEffect, useState } from 'react';
import { Toaster } from 'react-hot-toast';
import { StellarWalletsKit } from '@creit-tech/stellar-wallets-kit/sdk';
import { SwkAppDarkTheme, KitEventType } from '@creit-tech/stellar-wallets-kit/types';
import { defaultModules } from '@creit-tech/stellar-wallets-kit/modules/utils';
import { ConnectButton } from './components/ConnectButton';
import { VaultInterface } from './components/VaultInterface';
import './App.css';

// Initialize Stellar Wallets Kit
StellarWalletsKit.init({
  theme: SwkAppDarkTheme,
  modules: defaultModules(),
});

function App() {
  const [walletAddress, setWalletAddress] = useState('');
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    // Listen for wallet connection events
    StellarWalletsKit.on(KitEventType.STATE_UPDATED, (event) => {
      const address = event.payload.address || '';
      setWalletAddress(address);
      setIsConnected(!!address);
    });

    // Check if already connected
    StellarWalletsKit.getAddress()
      .then(({ address }) => {
        if (address) {
          setWalletAddress(address);
          setIsConnected(true);
        }
      })
      .catch(() => {
        // Not connected yet
      });
  }, []);

  return (
    <div className="app">
      <Toaster
        position="top-right"
        toastOptions={{
          duration: 5000,
          style: {
            background: '#1f2937',
            color: '#f9fafb',
            border: '1px solid rgba(255, 255, 255, 0.1)',
          },
          success: {
            iconTheme: {
              primary: '#10b981',
              secondary: '#f9fafb',
            },
          },
          error: {
            iconTheme: {
              primary: '#ef4444',
              secondary: '#f9fafb',
            },
          },
        }}
      />

      <header className="app-header">
        <div className="header-content">
          <div className="logo">
            <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
              <rect width="32" height="32" rx="8" fill="url(#gradient)" />
              <circle cx="16" cy="16" r="8" stroke="white" strokeWidth="2" fill="none" />
              <circle cx="16" cy="16" r="4" fill="white" />
              <defs>
                <linearGradient id="gradient" x1="0" y1="0" x2="32" y2="32">
                  <stop offset="0%" stopColor="#3b82f6" />
                  <stop offset="100%" stopColor="#2563eb" />
                </linearGradient>
              </defs>
            </svg>
            <div className="logo-text">
              <h1>Blend Vault</h1>
              <p>Max Yield On USDC</p>
            </div>
          </div>

          <ConnectButton address={walletAddress} isConnected={isConnected} />
        </div>
      </header>

      <main className="app-main">
        <div className="main-content">
          <div className="intro-section">
            <h2>USDC Auto-Compounding Vault</h2>
            <p>
              Deposit your USDC to earn yield automatically through the Blend Protocol's Yield Box
              Pool. Your deposits are supplied to Blend and earn interest continuously.
            </p>
          </div>

          <VaultInterface userAddress={walletAddress} isConnected={isConnected} />
        </div>
      </main>

      <footer className="app-footer">
        <div className="footer-content">
          <div className="footer-links">
            <a href="https://stellar.expert/explorer/public/contract/CCZWCNTCTHO3FE6YCYX6YYWFR3B3BEVICD42RZZFMWSPDEIFPQYW4IHA" target="_blank">
              View Contract
            </a>
            <a href="https://github.com/jamesbachini/Blend-Vault" target="_blank">
              Github
            </a>
            <a href="https://blend.capital" target="_blank">
              Blend Protocol
            </a>
            <a href="https://stellar.org" target="_blank">
              Stellar Network
            </a>
          </div>
          <p className="footer-disclaimer">
            This decentralized application is for experimentation purposes only and should not be used as an investment tool. Code is unaudited, any funds deposited risk being lost and unrecoverable.
          </p>
        </div>
      </footer>
    </div>
  );
}

export default App;
