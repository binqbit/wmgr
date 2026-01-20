import { Contract, JsonRpcProvider, parseEther, parseUnits } from 'ethers';
import { getEvmNetworkConfig } from '../config/evmNetworks.js';

const ERC20_ABI = [
  'function decimals() view returns (uint8)',
  'function symbol() view returns (string)',
  'function transfer(address to, uint256 amount) returns (bool)',
];

export function createEvmProvider(network, rpcOverride) {
  const { chainId, rpcUrl, name } = getEvmNetworkConfig(network, rpcOverride);
  const provider = new JsonRpcProvider(rpcUrl, chainId);
  return { provider, networkName: name, chainId, rpcUrl };
}

function maybeGasPrice(gwei) {
  if (!gwei) return undefined;
  return parseUnits(String(gwei), 'gwei');
}

function maybeGasLimit(limit) {
  if (!limit) return undefined;
  const n = BigInt(limit);
  if (n <= 0n) throw new Error('gasLimit must be > 0');
  return n;
}

export async function transferEth({ provider, signer, to, amount, gasPrice, gasLimit }) {
  const txReq = {
    to,
    value: parseEther(String(amount)),
  };
  const gp = maybeGasPrice(gasPrice);
  const gl = maybeGasLimit(gasLimit);
  if (gp) txReq.gasPrice = gp;
  if (gl) txReq.gasLimit = gl;

  const tx = await signer.sendTransaction(txReq);
  const receipt = await tx.wait();
  return { hash: tx.hash, receipt };
}

export async function transferErc20({ provider, signer, token, to, amount, decimals, gasPrice, gasLimit }) {
  const contract = new Contract(token, ERC20_ABI, signer);
  let tokenDecimals = decimals;
  if (tokenDecimals === undefined || tokenDecimals === null) {
    tokenDecimals = await contract.decimals();
  }
  const amt = parseUnits(String(amount), tokenDecimals);
  const overrides = {};
  const gp = maybeGasPrice(gasPrice);
  const gl = maybeGasLimit(gasLimit);
  if (gp) overrides.gasPrice = gp;
  if (gl) overrides.gasLimit = gl;

  const tx = await contract.transfer(to, amt, overrides);
  const receipt = await tx.wait();
  return { hash: tx.hash, receipt, decimals: tokenDecimals };
}

export async function getErc20Meta({ provider, token }) {
  const contract = new Contract(token, ERC20_ABI, provider);
  const [decimals, symbol] = await Promise.all([
    contract.decimals(),
    contract.symbol().catch(() => undefined),
  ]);
  return { decimals, symbol };
}
