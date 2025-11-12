import { useEffect, useState } from 'react';
import { Toaster } from 'react-hot-toast';
import { StellarWalletsKit } from '@creit-tech/stellar-wallets-kit/sdk';
import { SwkAppDarkTheme, KitEventType } from '@creit-tech/stellar-wallets-kit/types';
import { defaultModules } from '@creit-tech/stellar-wallets-kit/modules/utils';
import { ConnectButton } from './components/ConnectButton';
import { VaultInterface } from './components/VaultInterface';
import { StatsBar } from './components/StatsBar';
import { VAULT_CONTRACT_ID } from './utils/stellar';
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
            background: '#212328',
            color: '#FFFFFF',
            border: '1px solid rgba(255, 255, 255, 0.08)',
            fontFamily: '"DM Sans", Roboto',
            fontWeight: 500,
          },
          success: {
            iconTheme: {
              primary: '#36B04A',
              secondary: '#FFFFFF',
            },
          },
          error: {
            iconTheme: {
              primary: '#FF3366',
              secondary: '#FFFFFF',
            },
          },
        }}
      />

      <header className="app-header">
        <div className="header-content">
          <div className="logo">
            <img src="/vault-icon.svg" alt="Blend Vault" style={{ height: '32px', width: 'auto' }} />
            <div className="logo-text">
              <h1>BLEND VAULT</h1>
              <p>USDC YIELD STRATEGY</p>
            </div>
          </div>

          <ConnectButton address={walletAddress} isConnected={isConnected} />
        </div>
      </header>

      <StatsBar />

      <main className="app-main">
        <div className="main-content">
          <div className="intro-section">
            <h2>USDC AUTO-COMPOUNDING VAULT</h2>
            <p>
              Deposit USDC on Stellar to compound yield through Blend's <a href="https://mainnet.blend.capital/dashboard/?poolId=CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS" target="_blank">Yield Box</a> Pool
            </p>
          </div>

          <VaultInterface userAddress={walletAddress} isConnected={isConnected} />
        </div>
      </main>

      <footer className="app-footer">
        <div className="footer-content">
          <div className="footer-links">
            <a href={`https://stellar.expert/explorer/public/contract/${VAULT_CONTRACT_ID}`} target="_blank">
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
