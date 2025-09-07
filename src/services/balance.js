import { PublicKey } from '@solana/web3.js';
import { getAssociatedTokenAddress, getAccount, getMint } from '@solana/spl-token';
import { formatIntegerAmount } from '../utils/amount.js';

export async function getBalances({ connection, owner, mint }) {
  const ownerPk = owner instanceof PublicKey ? owner : new PublicKey(owner);

  // SOL
  const lamports = await connection.getBalance(ownerPk);
  const sol = formatIntegerAmount(BigInt(lamports), 9);

  // USDC (SPL)
  const mintPk = new PublicKey(mint);
  let decimals = 6; // default USDC decimals; fallback if mint missing on RPC
  try {
    const mintInfo = await getMint(connection, mintPk);
    decimals = mintInfo.decimals;
  } catch (_) {
    // Mint not found on this RPC (e.g., local validator). Treat as zero balance.
    decimals = 6;
  }
  let usdcInt = 0n;
  try {
    const ata = await getAssociatedTokenAddress(mintPk, ownerPk, true);
    const account = await getAccount(connection, ata);
    // amount is a bigint of raw token units
    usdcInt = BigInt(account.amount.toString());
  } catch (_) {
    // no ATA or not found -> zero balance
    usdcInt = 0n;
  }
  const usdc = formatIntegerAmount(usdcInt, decimals);

  return {
    address: ownerPk.toBase58(),
    solLamports: lamports,
    sol,
    usdcRaw: usdcInt,
    usdc,
    usdcDecimals: decimals,
  };
}
