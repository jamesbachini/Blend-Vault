0x05-01> Separate the main contract logic and the unit tests into two files lib.rs and test.rs

0x05-02> Create a build and deployment script to deploy this to Stellar mainnet

0x05-03> Generate the complete, production-ready source code for a frontend application that allows users to interact with the blend vault in contracts/src/lib.rs

I've deployed the contract to Stellar mainnet at this Contract ID:
CDORATDMBHHTWMAQMAFS2XL76SGRHW5PNMN25SIK4FB4UXYBGXRNRTBN

Create a frontend for this contract which provides a simple, elegant user interface enabling the user to approve spend of USDC, deposit it into the vault, check both USDC in wallet balance and their balance of USDC in the pool (this might be tricky because I don't want to show their shares I want to know how much USDC they deposited + the interest earnt), withdraw funds back to their wallet.

The user interface should provide a modern, clean, mobile-responsive, dark by default, intuitive UI. It should be comparable with other leading DeFi applications.

Do not fake anything or create mocks or placeholders, this needs to be suitable for production.

UI/UX & Production Standards
Theme: Dark mode by default. Use a clean, modern aesthetic (dark greys, white/blue text, accented buttons) comparable to leading DeFi apps.

Responsiveness: The layout must be fully mobile-responsive.

No Mocks: The code must be production-ready. Do not fake any data or use placeholders. All values must be fetched live from the blockchain or calculated from contract calls.

Loading States: All buttons (Approve, Deposit, Withdraw) must show a loading spinner and be disabled while a transaction is pending. Balance displays should show a skeleton loader while fetching.

Error Handling: Provide clean toast notifications for all possible outcomes (e.g., "Transaction successful," "User denied transaction," "Error: Insufficient funds").

Code Quality: Generate clean, modular code. Separate components logically (e.g., ConnectButton.tsx, Vault.tsx, BalanceDisplay.tsx). Use appropriate TypeScript types based on the ABI.

Use Creit-Tech's stellar wallet sdk for the wallet connection. Example code below:

import './App.css';
import React, { useEffect, useRef, useState } from 'react';

import { StellarWalletsKit } from "@creit-tech/stellar-wallets-kit/sdk";
import { SwkAppDarkTheme, KitEventType } from "@creit-tech/stellar-wallets-kit/types";
import { defaultModules } from '@creit-tech/stellar-wallets-kit/modules/utils';
import * as StellarSdk from '@stellar/stellar-sdk';

StellarWalletsKit.init({
  theme: SwkAppDarkTheme,
  modules: defaultModules(),
});


function App() {
    const [walletAddress, setWalletAddress] = useState('Disconnected');
    const [explorerLink, setExplorerLink] = useState('');
    const buttonWrapperRef = useRef(null);
    
    useEffect(() => {
        const buttonWrapper = buttonWrapperRef.current;
        if (buttonWrapper) {
          StellarWalletsKit.createButton(buttonWrapper);
        }
        
        StellarWalletsKit.on(KitEventType.STATE_UPDATED, event => {
          console.log('EVENT LOG: ', event, event.payload.address);
          setWalletAddress(event.payload.address || 'Disconnected');
        });
    }, []); 

    const sendTx = async () => {
      try {
        const DESTINATION_ADDRESS = 'GB5AT3W7YT5OOF7HFDIFRM6AS2HXQF7QOL47ZHMGETN4P63476Z6DQ43';
        const { address } = await StellarWalletsKit.getAddress();
        const server = new StellarSdk.Horizon.Server('https://horizon-testnet.stellar.org');
        const sourceAccount = await server.loadAccount(address);
        const transaction = new StellarSdk.TransactionBuilder(sourceAccount, {
          fee: StellarSdk.BASE_FEE,
          networkPassphrase: StellarSdk.Networks.TESTNET
        })
          .addOperation(
            StellarSdk.Operation.payment({
              destination: DESTINATION_ADDRESS,
              asset: StellarSdk.Asset.native(),
              amount: '1'
            })
          )
          .setTimeout(180)
          .build();
        const {signedTxXdr} = await StellarWalletsKit.signTransaction(transaction.toXDR(), {
          networkPassphrase: StellarSdk.Networks.TESTNET,
          address,
        });
        const transactionToSubmit = StellarSdk.TransactionBuilder.fromXDR(
          signedTxXdr,
          StellarSdk.Networks.TESTNET
        );
        const response = await server.submitTransaction(transactionToSubmit);
        console.log('Transaction successful!', response);
        const blockExplorer = `https://stellar.expert/explorer/testnet/tx/${response.hash}`;
        setExplorerLink(blockExplorer);
        alert(`Transaction sent successfully!`);
      } catch (error) {
        console.error('Transaction failed:', error);
        alert('Transaction failed: ' + error.message);
      }
    }

  return (
    <div className="App">
      <header className="App-header">
        <div className="App-address">{walletAddress}</div>
        <div ref={buttonWrapperRef} id="buttonWrapper"></div>
        <button onClick={sendTx}>Send Test Tx</button>
        <div className="App-explorer-link">
          <a href={explorerLink} target="_blank" rel="noopener noreferrer">
            {explorerLink}
          </a>
        </div>
        
      </header>
    </div>
  );
}

export default App;