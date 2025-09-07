import { LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { parseAmountToInteger } from '../utils/amount.js';

export async function transferSol({ connection, from, to, amount }) {
  const toPk = new PublicKey(to);
  const lamports = parseAmountToInteger(String(amount), 9); // SOL has 9 decimals

  const tx = new Transaction().add(
    SystemProgram.transfer({ fromPubkey: from.publicKey, toPubkey: toPk, lamports: Number(lamports) })
  );

  const sig = await connection.sendTransaction(tx, [from], { skipPreflight: false });
  // Confirm at the connection's configured commitment
  const latest = await connection.getLatestBlockhash();
  await connection.confirmTransaction({ signature: sig, ...latest });
  return sig;
}

