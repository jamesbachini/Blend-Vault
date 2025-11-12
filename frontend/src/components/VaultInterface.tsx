import React, { useState, useEffect, useMemo } from 'react';
import toast from 'react-hot-toast';
import { BalanceDisplay } from './BalanceDisplay';
import { ActionButton } from './ActionButton';
import * as USDCContract from '../contracts/usdc';
import * as VaultContract from '../contracts/vault';
import { parseUSDC, formatUSDC } from '../utils/format';
import { useVaultRewards } from '../hooks/useVaultRewards';
import './VaultInterface.css';

interface VaultInterfaceProps {
  userAddress: string;
  isConnected: boolean;
}

const MIN_COMPOUND_BLND = 0.0001;

const formatBlndAmount = (value: number): string => {
  const absValue = Math.abs(value);
  const maximumFractionDigits = absValue < 1 ? 4 : 2;
  const minimumFractionDigits = value === 0 ? 0 : absValue < 1 ? 2 : 0;

  return value.toLocaleString('en-US', {
    minimumFractionDigits,
    maximumFractionDigits,
  });
};

export const VaultInterface: React.FC<VaultInterfaceProps> = ({ userAddress, isConnected }) => {
  const [walletBalance, setWalletBalance] = useState<bigint | null>(null);
  const [vaultBalance, setVaultBalance] = useState<bigint | null>(null);
  const [allowance, setAllowance] = useState<bigint>(BigInt(0));
  const [isLoadingBalances, setIsLoadingBalances] = useState(false);
  const [isApproving, setIsApproving] = useState(false);
  const [isDepositing, setIsDepositing] = useState(false);
  const [isWithdrawing, setIsWithdrawing] = useState(false);
  const [isCompounding, setIsCompounding] = useState(false);
  const [depositAmount, setDepositAmount] = useState('');
  const [withdrawAmount, setWithdrawAmount] = useState('');
  const {
    pendingBlnd,
    isLoading: isLoadingPendingBlnd,
    error: pendingBlndError,
    refresh: refreshPendingBlnd,
  } = useVaultRewards({ enabled: isConnected });
  const hasCompoundableRewards = useMemo(
    () =>
      !isLoadingPendingBlnd &&
      pendingBlnd !== null &&
      pendingBlnd >= MIN_COMPOUND_BLND,
    [isLoadingPendingBlnd, pendingBlnd]
  );

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
      // Simple withdraw - just pass the USDC amount directly
      // The contract handles all the share calculations internally
      const txHash = await VaultContract.withdraw(amount, userAddress);

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

  const handleCompound = async () => {
    if (pendingBlnd !== null && !hasCompoundableRewards) {
      toast.error('Not enough BLND to compound yet (need at least 0.0001 BLND).');
      return;
    }

    setIsCompounding(true);
    try {
      const txHash = await VaultContract.compound(userAddress);
      toast.success(
        <div>
          Compound successful!{' '}
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
      await fetchBalances();
      await refreshPendingBlnd(false);
    } catch (error: any) {
      console.error('Compound error:', error);
      if (error.message?.includes('User declined')) {
        toast.error('Transaction was cancelled');
      } else {
        toast.error(`Compound failed: ${error.message || 'Unknown error'}`);
      }
    } finally {
      setIsCompounding(false);
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

  const needsApproval = !!depositAmount && parseUSDC(depositAmount) > allowance;
  const pendingBlndDisplay = useMemo(() => {
    if (isLoadingPendingBlnd) {
      return 'Loading...';
    }
    if (pendingBlnd === null) {
      return '--';
    }
    return `${formatBlndAmount(pendingBlnd)} BLND`;
  }, [isLoadingPendingBlnd, pendingBlnd]);

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
                disabled={isDepositing || isWithdrawing || isCompounding}
                variant="secondary"
              >
                Approve USDC
              </ActionButton>
            )}
            <ActionButton
              onClick={handleDeposit}
              isLoading={isDepositing}
              disabled={
                isApproving || isWithdrawing || isCompounding || !depositAmount || needsApproval
              }
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
            disabled={isApproving || isDepositing || isCompounding || !withdrawAmount}
          >
            Withdraw
          </ActionButton>

          <div className="compound-section">
            <h3 className="action-title">Compound</h3>
            <p className="action-description">
              Compound BLND rewards back in to the USDC vault
            </p>
            <div className="pending-blnd-card">
              <div className="pending-blnd-details">
                <span className="pending-blnd-label">BLND ready to compound</span>
                <span className="pending-blnd-value">{pendingBlndDisplay}</span>
              </div>
              {!isLoadingPendingBlnd && pendingBlndError && (
                <span className="pending-blnd-error">Unable to load BLND rewards.</span>
              )}
              {!pendingBlndError && !isLoadingPendingBlnd && pendingBlnd !== null && pendingBlnd < MIN_COMPOUND_BLND && (
                <span className="pending-blnd-warning">
                  Not enough BLND to compound yet (need ≥ 0.0001 BLND).
                </span>
              )}
            </div>
            <ActionButton
              onClick={handleCompound}
              isLoading={isCompounding}
              disabled={
                isApproving ||
                isDepositing ||
                isWithdrawing ||
                (pendingBlnd !== null && !hasCompoundableRewards)
              }
            >
              Compound
            </ActionButton>
          </div>
        </div>
      </div>
    </div>
  );
};
