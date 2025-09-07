export function parseAmountToInteger(amountStr, decimals) {
  if (typeof amountStr !== 'string') amountStr = String(amountStr);
  const trimmed = amountStr.trim();
  if (!/^\d*(\.\d*)?$/.test(trimmed)) {
    throw new Error(`Invalid amount format: ${amountStr}`);
  }
  const [leftRaw, rightRaw = ''] = trimmed.split('.');
  const left = leftRaw.replace(/^0+(?=\d)/, '');
  let right = rightRaw.padEnd(decimals, '0').slice(0, decimals);
  const digits = `${left || '0'}${right}`.replace(/^0+(?=\d)/, '');
  return BigInt(digits || '0');
}

export function formatIntegerAmount(intAmount, decimals) {
  const n = BigInt(intAmount || 0);
  const factor = BigInt(10) ** BigInt(decimals);
  const whole = n / factor;
  const frac = n % factor;
  if (frac === 0n) return whole.toString();
  const fracStr = frac.toString().padStart(decimals, '0').replace(/0+$/,'');
  return `${whole.toString()}.${fracStr}`;
}
