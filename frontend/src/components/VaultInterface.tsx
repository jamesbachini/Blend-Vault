import React, { useState, useEffect } from 'react';
import toast from 'react-hot-toast';
import { BalanceDisplay } from './BalanceDisplay';
import { ActionButton } from './ActionButton';
import * as USDCContract from '../contracts/usdc';
import * as VaultContract from '../contracts/vault';
import { parseUSDC, formatUSDC } from '../utils/format';
import './VaultInterface.css';

interface VaultInterfaceProps {
  userAddress: string;
  isConnected: boolean;
}

export const VaultInterface: React.FC<VaultInterfaceProps> = ({ userAddress, isConnected }) => {
  const [walletBalance, setWalletBalance] = useState<bigint | null>(null);
  const [vaultBalance, setVaultBalance] = useState<bigint | null>(null);
  const [allowance, setAllowance] = useState<bigint>(BigInt(0));
  const [isLoadingBalances, setIsLoadingBalances] = useState(false);
  const [isApproving, setIsApproving] = useState(false);
  const [isDepositing, setIsDepositing] = useState(false);
  const [isWithdrawing, setIsWithdrawing] = useState(false);
  const [depositAmount, setDepositAmount] = useState('');
  const [withdrawAmount, setWithdrawAmount] = useState('');

  // Fetch balances
  const fetchBalances = async () => {
    if (!isConnected || !userAddress) return;

    setIsLoadingBalances(true);
    try {
      // Fetch wallet USDC balance
      const usdcBalance = await USDCContract.getBalance(userAddress);
      setWalletBalance(usdcBalance);

      // Fetch vault shares and convert to USDC
      const shares = await VaultContract.getShareBalance(userAddress);
      if (shares > BigInt(0)) {
        const assets = await VaultContract.convertToAssets(shares, userAddress);
        setVaultBalance(assets);
      } else {
        setVaultBalance(BigInt(0));
      }

      // Fetch allowance
      const currentAllowance = await USDCContract.getAllowance(userAddress);
      setAllowance(currentAllowance);
    } catch (error) {
      console.error('Error fetching balances:', error);
      toast.error('Failed to fetch balances');
    } finally {
      setIsLoadingBalances(false);
    }
  };

  useEffect(() => {
    fetchBalances();
    // Refresh balances every 30 seconds
    const interval = setInterval(fetchBalances, 30000);
    return () => clearInterval(interval);
  }, [userAddress, isConnected]);

  const handleApprove = async () => {
    if (!depositAmount || parseFloat(depositAmount) <= 0) {
      toast.error('Please enter a valid amount');
      return;
    }

    setIsApproving(true);
    try {
      const amount = parseUSDC(depositAmount);
      const txHash = await USDCContract.approve(amount, userAddress);
      toast.success(
        <div>
          Approval successful!{' '}
          <a
            href={`https://stellar.expert/explorer/public/tx/${txHash}`}
            target="_blank"
            rel="noopener noreferrer"
            style={{ textDecoration: 'underline' }}
          >
            View transaction
          </a>
        </div>
      );
      // Refresh allowance
      const newAllowance = await USDCContract.getAllowance(userAddress);
      setAllowance(newAllowance);
    } catch (error: any) {
      console.error('Approval error:', error);
      if (error.message?.includes('User declined')) {
        toast.error('Transaction was cancelled');
      } else {
        toast.error(`Approval failed: ${error.message || 'Unknown error'}`);
      }
    } finally {
      setIsApproving(false);
    }
  };

  const handleDeposit = async () => {
    if (!depositAmount || parseFloat(depositAmount) <= 0) {
      toast.error('Please enter a valid amount');
      return;
    }

    const amount = parseUSDC(depositAmount);

    if (walletBalance !== null && amount > walletBalance) {
      toast.error('Insufficient USDC balance');
      return;
    }

    if (amount > allowance) {
      toast.error('Insufficient allowance. Please approve first.');
      return;
    }

    setIsDepositing(true);
    try {
      const txHash = await VaultContract.deposit(amount, userAddress);
      toast.success(
        <div>
          Deposit successful!{' '}
          <a
            href={`https://stellar.expert/explorer/public/tx/${txHash}`}
            target="_blank"
            rel="noopener noreferrer"
            style={{ textDecoration: 'underline' }}
          >
            View transaction
          </a>
        </div>
      );
      setDepositAmount('');
      await fetchBalances();
    } catch (error: any) {
      console.error('Deposit error:', error);
      if (error.message?.includes('User declined')) {
        toast.error('Transaction was cancelled');
      } else {
        toast.error(`Deposit failed: ${error.message || 'Unknown error'}`);
      }
    } finally {
      setIsDepositing(false);
    }
  };

  const handleWithdraw = async () => {
    if (!withdrawAmount || parseFloat(withdrawAmount) <= 0) {
      toast.error('Please enter a valid amount');
      return;
    }

    const amount = parseUSDC(withdrawAmount);

    if (vaultBalance !== null && amount > vaultBalance) {
      toast.error('Insufficient vault balance');
      return;
    }

    setIsWithdrawing(true);
    try {
      // Get fresh data from chain RIGHT before withdrawal
      const userShares = await VaultContract.getShareBalance(userAddress);

      if (userShares === 0n) {
        toast.error('You have no shares to withdraw');
        setIsWithdrawing(false);
        return;
      }

      // Get vault's total state to verify calculations
      const totalSupply = await VaultContract.getTotalSupply(userAddress);
      const totalAssets = await VaultContract.getTotalAssets(userAddress);

      console.log('=== Withdrawal Debug ===');
      console.log('User shares:', userShares.toString(), '(', (Number(userShares) / 10_000_000).toFixed(7), ')');
      console.log('Total supply:', totalSupply.toString(), '(', (Number(totalSupply) / 10_000_000).toFixed(7), ')');
      console.log('Total assets:', totalAssets.toString(), '(', (Number(totalAssets) / 10_000_000).toFixed(7), 'USDC)');
      console.log('Withdrawal amount:', amount.toString(), '(', (Number(amount) / 10_000_000).toFixed(7), 'USDC)');

      // Calculate user's actual asset value
      const userAssets = (userShares * totalAssets) / totalSupply;
      console.log('User asset value:', userAssets.toString(), '(', (Number(userAssets) / 10_000_000).toFixed(7), 'USDC)');

      // Check share price using JavaScript numbers to avoid integer overflow
      const sharePriceFloat = Number(totalAssets) / Number(totalSupply);
      console.log('Share price:', sharePriceFloat.toFixed(9), 'USDC per share');

      // Validate
      if (amount > userAssets) {
        toast.error(`Cannot withdraw ${(Number(amount) / 10_000_000).toFixed(4)} USDC. You only have ${(Number(userAssets) / 10_000_000).toFixed(4)} USDC.`);
        setIsWithdrawing(false);
        return;
      }

      if (amount > totalAssets) {
        toast.error(`Vault only has ${(Number(totalAssets) / 10_000_000).toFixed(4)} USDC total. Cannot withdraw ${(Number(amount) / 10_000_000).toFixed(4)} USDC.`);
        setIsWithdrawing(false);
        return;
      }

      let txHash: string;

      // With inflation attack protection, vaults can have very high share counts
      // For small total asset amounts (< 1 USDC), ALWAYS redeem all shares
      if (totalAssets < 10_000_000n) {
        console.log('==> Small vault detected, forcing full withdrawal of ALL shares');
        txHash = await VaultContract.redeem(userShares, userAddress);
      } else if (amount >= (userAssets * 90n) / 100n) {
        // If withdrawing 90%+ of balance, redeem ALL shares to avoid rounding issues
        console.log('==> Near-full withdrawal: redeeming ALL shares');
        txHash = await VaultContract.redeem(userShares, userAddress);
      } else {
        // Partial withdrawal: calculate proportional shares
        // shares = (amount * userShares) / userAssets
        const sharesToRedeem = (amount * userShares) / userAssets;

        console.log('==> Partial: trying to redeem', sharesToRedeem.toString(), 'shares (', (Number(sharesToRedeem) / 10_000_000).toFixed(7), ')');

        // CRITICAL: Validate with the contract's formula to ensure no rounding to 0
        // The contract will use preview_redeem which might round differently
        // For safety, if calculated shares would give < 0.001 USDC, use full withdrawal
        const assetsFromRedeem = (sharesToRedeem * totalAssets) / totalSupply;
        console.log('Calculated assets from redeem:', assetsFromRedeem.toString());

        if (assetsFromRedeem < 10000n) {
          toast.error(
            `⚠️ Cannot withdraw this amount due to rounding (would get ${(Number(assetsFromRedeem) / 10_000_000).toFixed(7)} USDC). Please withdraw everything using MAX button.`,
            { duration: 7000 }
          );
          setIsWithdrawing(false);
          return;
        }

        txHash = await VaultContract.redeem(sharesToRedeem, userAddress);
      }

      toast.success(
        <div>
          Withdrawal successful!{' '}
          <a
            href={`https://stellar.expert/explorer/public/tx/${txHash}`}
            target="_blank"
            rel="noopener noreferrer"
            style={{ textDecoration: 'underline' }}
          >
            View transaction
          </a>
        </div>
      );
      setWithdrawAmount('');
      await fetchBalances();
    } catch (error: any) {
      console.error('Withdraw error:', error);
      if (error.message?.includes('User declined')) {
        toast.error('Transaction was cancelled');
      } else {
        toast.error(`Withdrawal failed: ${error.message || 'Unknown error'}`);
      }
    } finally {
      setIsWithdrawing(false);
    }
  };

  const setMaxDeposit = () => {
    if (walletBalance !== null) {
      setDepositAmount(formatUSDC(walletBalance));
    }
  };

  const setMaxWithdraw = async () => {
    if (!isConnected || !userAddress) return;

    try {
      // Get user's actual share balance and redeem all shares
      // This avoids rounding issues with asset amounts
      const shares = await VaultContract.getShareBalance(userAddress);
      if (shares > BigInt(0)) {
        const assets = await VaultContract.convertToAssets(shares, userAddress);
        setWithdrawAmount(formatUSDC(assets));
      }
    } catch (error) {
      console.error('Error getting max withdraw:', error);
      if (vaultBalance !== null) {
        setWithdrawAmount(formatUSDC(vaultBalance));
      }
    }
  };

  if (!isConnected) {
    return (
      <div className="vault-interface">
        <div className="connect-prompt">
          <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
            <circle cx="24" cy="24" r="20" stroke="currentColor" strokeWidth="2" />
            <path d="M24 16v16M16 24h16" stroke="currentColor" strokeWidth="2" />
          </svg>
          <h3>Connect Your Wallet</h3>
          <p>Please connect your wallet to interact with the Blend Vault</p>
        </div>
      </div>
    );
  }

  const needsApproval = !!depositAmount && parseUSDC(depositAmount) > allowance;

  return (
    <div className="vault-interface">
      <BalanceDisplay
        walletBalance={walletBalance}
        vaultBalance={vaultBalance}
        isLoading={isLoadingBalances}
      />

      <div className="vault-actions">
        <div className="action-section">
          <h3 className="action-title">Deposit USDC</h3>
          <p className="action-description">
            Deposit USDC into the vault to earn yield from Blend Protocol
          </p>

          <div className="input-group">
            <input
              type="number"
              className="amount-input"
              placeholder="0.00"
              value={depositAmount}
              onChange={(e) => setDepositAmount(e.target.value)}
              step="0.01"
              min="0"
            />
            <button className="max-button" onClick={setMaxDeposit}>
              MAX
            </button>
          </div>

          <div className="button-group">
            {needsApproval && (
              <ActionButton
                onClick={handleApprove}
                isLoading={isApproving}
                disabled={isDepositing || isWithdrawing}
                variant="secondary"
              >
                Approve USDC
              </ActionButton>
            )}
            <ActionButton
              onClick={handleDeposit}
              isLoading={isDepositing}
              disabled={isApproving || isWithdrawing || !depositAmount || needsApproval}
            >
              Deposit
            </ActionButton>
          </div>
        </div>

        <div className="action-divider" />

        <div className="action-section">
          <h3 className="action-title">Withdraw USDC</h3>
          <p className="action-description">
            Withdraw your USDC from the vault along with earned yield
          </p>

          <div className="input-group">
            <input
              type="number"
              className="amount-input"
              placeholder="0.00"
              value={withdrawAmount}
              onChange={(e) => setWithdrawAmount(e.target.value)}
              step="0.01"
              min="0"
            />
            <button className="max-button" onClick={setMaxWithdraw}>
              MAX
            </button>
          </div>

          <ActionButton
            onClick={handleWithdraw}
            isLoading={isWithdrawing}
            disabled={isApproving || isDepositing || !withdrawAmount}
          >
            Withdraw
          </ActionButton>
        </div>
      </div>
    </div>
  );
};
