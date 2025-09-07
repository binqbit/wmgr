import { Connection } from '@solana/web3.js';

export function createConnection(rpcUrl, commitment = 'confirmed') {
  return new Connection(rpcUrl, { commitment });
}

