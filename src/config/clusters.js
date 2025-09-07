const DEFAULTS = {
  'mainnet-beta': {
    url: 'https://api.mainnet-beta.solana.com',
  },
  devnet: {
    url: 'https://api.devnet.solana.com',
  },
  testnet: {
    url: 'https://api.testnet.solana.com',
  },
};

// Common USDC mint addresses
// mainnet-beta: EPjF... is canonical USDC. devnet uses 4zMMC... (popular faucet variant)
const USDC_MINTS = {
  'mainnet-beta': 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  devnet: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
  testnet: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
};

export function getClusterConfig(cluster = 'mainnet-beta', overrideUrl) {
  console.log(`Using cluster: ${cluster}, RPC: ${overrideUrl || '(default)'}`);
  const cfg = DEFAULTS[cluster];
  if (!cfg) throw new Error(`Unknown cluster: ${cluster}`);
  return { url: overrideUrl || cfg.url };
}

export function getUsdcMintForCluster(cluster = 'devnet') {
  const mint = USDC_MINTS[cluster];
  if (!mint) throw new Error(`No USDC mint configured for cluster: ${cluster}`);
  return mint;
}

