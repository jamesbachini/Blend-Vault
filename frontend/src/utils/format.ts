// USDC has 7 decimals on Stellar
const USDC_DECIMALS = 7;

// Format USDC amount from contract (stroop) to human readable
export function formatUSDC(amount: bigint): string {
  const divisor = BigInt(10 ** USDC_DECIMALS);
  const whole = amount / divisor;
  const fraction = amount % divisor;

  const fractionStr = fraction.toString().padStart(USDC_DECIMALS, '0');

  // Remove trailing zeros
  const trimmedFraction = fractionStr.replace(/0+$/, '');

  if (trimmedFraction === '') {
    return whole.toString();
  }

  return `${whole}.${trimmedFraction}`;
}

// Parse human readable USDC to contract amount (stroop)
export function parseUSDC(amount: string): bigint {
  const parts = amount.split('.');
  const whole = parts[0] || '0';
  const fraction = parts[1] || '0';

  // Pad or trim fraction to USDC_DECIMALS
  const paddedFraction = fraction.padEnd(USDC_DECIMALS, '0').slice(0, USDC_DECIMALS);

  const wholeAmount = BigInt(whole) * BigInt(10 ** USDC_DECIMALS);
  const fractionAmount = BigInt(paddedFraction);

  return wholeAmount + fractionAmount;
}

// Format address for display (truncate middle)
export function formatAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

// Format USDC with thousands separators
export function formatUSDCWithCommas(amount: bigint): string {
  const formatted = formatUSDC(amount);
  const parts = formatted.split('.');
  parts[0] = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return parts.join('.');
}
