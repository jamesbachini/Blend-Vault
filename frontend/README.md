# Blend Vault Frontend

A production-ready React frontend for interacting with the Blend Vault smart contract on Stellar. This application allows users to deposit USDC into the vault and earn yield through the Blend Protocol.

## Features

- **Wallet Connection**: Connect using any Stellar wallet via @creit-tech/stellar-wallets-kit
- **Balance Display**: View your USDC wallet balance and vault balance (deposits + yield)
- **Approve & Deposit**: Approve USDC spending and deposit into the vault
- **Withdraw**: Withdraw your USDC from the vault including earned yield
- **Real-time Updates**: Balances refresh automatically every 30 seconds
- **Mobile Responsive**: Fully responsive design optimized for all devices
- **Dark Theme**: Modern, clean dark UI with smooth animations
- **Transaction Notifications**: Toast notifications with transaction links

## Tech Stack

- **React 18** with TypeScript
- **Vite** for fast builds and development
- **Stellar SDK** for blockchain interactions
- **Creit-Tech Stellar Wallets Kit** for wallet connections
- **React Hot Toast** for notifications

## Prerequisites

- Node.js 18+ and npm/yarn
- A Stellar wallet (Freighter, xBull, etc.)
- USDC on Stellar mainnet

## Installation

1. Install dependencies:

```bash
npm install
```

2. Start the development server:

```bash
npm run dev
```

The app will be available at http://localhost:5173

## Building for Production

```bash
npm run build
```

The built files will be in the `dist/` directory.

## Preview Production Build

```bash
npm run preview
```

## Contract Addresses

- **Vault Contract**: `CCZWCNTCTHO3FE6YCYX6YYWFR3B3BEVICD42RZZFMWSPDEIFPQYW4IHA`
- **USDC Token**: `CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75`
- **Network**: Stellar Mainnet

## Project Structure

```
frontend/
├── src/
│   ├── components/         # React components
│   │   ├── ConnectButton.tsx
│   │   ├── BalanceDisplay.tsx
│   │   ├── ActionButton.tsx
│   │   └── VaultInterface.tsx
│   ├── contracts/          # Contract interaction logic
│   │   ├── vault.ts
│   │   └── usdc.ts
│   ├── utils/              # Utility functions
│   │   ├── stellar.ts
│   │   └── format.ts
│   ├── types/              # TypeScript types
│   │   └── index.ts
│   ├── App.tsx             # Main app component
│   ├── App.css             # Global styles
│   └── main.tsx            # Entry point
├── public/                 # Static assets
├── index.html              # HTML template
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## How It Works

### Deposits

1. User enters amount and clicks "Approve USDC" (one-time per amount)
2. User clicks "Deposit" to transfer USDC to the vault
3. Vault supplies USDC to Blend Protocol
4. User receives vault shares representing their deposit

### Withdrawals

1. User enters amount and clicks "Withdraw"
2. Vault redeems shares and withdraws USDC from Blend
3. USDC (including yield) is transferred back to user's wallet

### Balance Calculation

- **Wallet Balance**: Direct query of USDC token balance
- **Vault Balance**: Vault shares converted to USDC using `convert_to_assets()`
  - This shows the current USDC value including earned yield
  - As yield accrues in Blend, the share value increases

## Development

The app uses:

- **Soroban RPC**: https://soroban-rpc.mainnet.stellar.gateway.fm
- **Horizon**: https://horizon.stellar.org

All contract interactions are simulated before signing to provide accurate fee estimates and catch errors early.

## Security Considerations

- Contract addresses are hardcoded to prevent phishing
- All transactions require user approval via wallet
- Input validation on amounts
- Error handling for all contract calls
- No private keys are ever handled by the application

## Troubleshooting

**"Transaction failed" errors**:
- Ensure you have enough XLM for transaction fees (~0.1 XLM recommended)
- Check that you have sufficient USDC balance
- Verify you've approved enough USDC allowance

**Balances not updating**:
- Balances automatically refresh every 30 seconds
- Manual refresh: reconnect your wallet

**Wallet connection issues**:
- Make sure your wallet extension is installed and unlocked
- Try refreshing the page
- Check that your wallet is connected to Stellar mainnet

## License

MIT

## Links

- [Blend Protocol](https://blend.capital)
- [Stellar Network](https://stellar.org)
- [View Contract on Stellar Expert](https://stellar.expert/explorer/public/contract/CCZWCNTCTHO3FE6YCYX6YYWFR3B3BEVICD42RZZFMWSPDEIFPQYW4IHA)
