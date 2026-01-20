// Default EVM network presets with public RPC endpoints.
// These are intentionally free/public RPCs; for production, override via --rpc.
export const EVM_NETWORKS = {
  mainnet: { chainId: 1, rpcUrl: 'https://cloudflare-eth.com' },
  sepolia: { chainId: 11155111, rpcUrl: 'https://rpc.sepolia.org' },
  holesky: { chainId: 17000, rpcUrl: 'https://ethereum-holesky.publicnode.com' },
  polygon: { chainId: 137, rpcUrl: 'https://polygon-rpc.com' },
  polygon_amoy: { chainId: 80002, rpcUrl: 'https://rpc-amoy.polygon.technology' },
  bsc: { chainId: 56, rpcUrl: 'https://bsc-dataseed.binance.org' },
  bsc_testnet: { chainId: 97, rpcUrl: 'https://data-seed-prebsc-1-s1.binance.org:8545' },
  avalanche: { chainId: 43114, rpcUrl: 'https://api.avax.network/ext/bc/C/rpc' },
  avalanche_fuji: { chainId: 43113, rpcUrl: 'https://api.avax-test.network/ext/bc/C/rpc' },
  optimism: { chainId: 10, rpcUrl: 'https://mainnet.optimism.io' },
  arbitrum: { chainId: 42161, rpcUrl: 'https://arb1.arbitrum.io/rpc' },
};

export function getEvmNetworkConfig(name = 'mainnet', overrideRpc) {
  const key = String(name || '').toLowerCase();
  const cfg = EVM_NETWORKS[key];
  if (!cfg) {
    throw new Error(`Unknown EVM network: ${name}`);
  }
  const rpcUrl = overrideRpc || cfg.rpcUrl;
  return { ...cfg, rpcUrl, name: key };
}
