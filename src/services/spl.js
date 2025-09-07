import { PublicKey, Transaction } from '@solana/web3.js';
import {
  getAssociatedTokenAddress,
  createAssociatedTokenAccountIdempotentInstruction,
  createTransferCheckedInstruction,
  getMint,
} from '@solana/spl-token';
import { parseAmountToInteger } from '../utils/amount.js';

export async function transferSplToken({ connection, from, to, amount, mint }) {
  const toOwner = new PublicKey(to);
  const mintPk = new PublicKey(mint);

  const mintInfo = await getMint(connection, mintPk);
  const decimals = mintInfo.decimals;

  const fromAta = await getAssociatedTokenAddress(mintPk, from.publicKey, true);
  const toAta = await getAssociatedTokenAddress(mintPk, toOwner, true);

  const amountInt = parseAmountToInteger(String(amount), decimals);

  const tx = new Transaction();
  tx.add(
    createAssociatedTokenAccountIdempotentInstruction(
      from.publicKey,
      fromAta,
      from.publicKey,
      mintPk
    ),
  );
  tx.add(
    createAssociatedTokenAccountIdempotentInstruction(
      from.publicKey,
      toAta,
      toOwner,
      mintPk
    ),
  );
  tx.add(
    createTransferCheckedInstruction(
      fromAta,
      mintPk,
      toAta,
      from.publicKey,
      amountInt,
      decimals
    ),
  );

  const sig = await connection.sendTransaction(tx, [from], { skipPreflight: false });
  const latest = await connection.getLatestBlockhash();
  await connection.confirmTransaction({ signature: sig, ...latest });
  return sig;
}

