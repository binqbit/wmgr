import { readFile } from 'fs/promises';
import { HDNodeWallet, Wallet } from 'ethers';
import { validateMnemonic } from 'bip39';
import { getMnemonicFromSvpi } from './svpi.js';
import { prompt, promptHidden } from '../utils/prompt.js';

export const DEFAULT_EVM_PATH = "m/44'/60'/0'/0/0";

function normalizePrivateKey(hex) {
  if (!hex) throw new Error('Private key is required');
  const trimmed = hex.trim().toLowerCase();
  const withoutPrefix = trimmed.startsWith('0x') ? trimmed.slice(2) : trimmed;
  if (!/^[0-9a-f]{64}$/.test(withoutPrefix)) {
    throw new Error('Private key must be 64 hex chars (with or without 0x prefix)');
  }
  return `0x${withoutPrefix}`;
}

export function walletFromPrivateKey(hex) {
  const pk = normalizePrivateKey(hex);
  return new Wallet(pk);
}

export async function walletFromPrivateKeyFile(filePath) {
  const buf = await readFile(filePath, 'utf8');
  return walletFromPrivateKey(buf);
}

export function walletFromMnemonic({ mnemonic, path = DEFAULT_EVM_PATH, passphrase = '' }) {
  if (!validateMnemonic(mnemonic)) {
    throw new Error('Invalid BIP39 mnemonic for EVM wallet');
  }
  // HDNodeWallet derives using secp256k1; passphrase is optional
  return HDNodeWallet.fromPhrase(mnemonic, passphrase, path);
}

export async function walletFromSvpi({ name, password, filePath, derivationPath = DEFAULT_EVM_PATH, seedPassphrase = '' }) {
  const mnemonic = await getMnemonicFromSvpi(name, password, filePath);
  return walletFromMnemonic({ mnemonic, path: derivationPath, passphrase: seedPassphrase });
}

export async function resolveEvmWallet({ privkey, privkeyFile, seed, derivationPath = DEFAULT_EVM_PATH, seedPassphrase = '', svpi, svpiName, svpiFile, svpiPassword }) {
  const provided = [privkey, privkeyFile, seed, svpi].filter(Boolean);
  if (provided.length === 0) {
    throw new Error('Provide one of --privkey, --privkey-file, --seed, or --svpi for EVM');
  }
  if (provided.length > 1) {
    throw new Error('Use only one of --privkey, --privkey-file, --seed, or --svpi for EVM');
  }
  if (privkey) return walletFromPrivateKey(privkey);
  if (privkeyFile) return walletFromPrivateKeyFile(privkeyFile);
  if (seed) return walletFromMnemonic({ mnemonic: seed, path: derivationPath, passphrase: seedPassphrase });
  // svpi flow
  const name = svpiName || await prompt('SVPI wallet name (EVM): ');
  const password = svpiPassword || await promptHidden('SVPI password: ');
  return walletFromSvpi({ name, password, filePath: svpiFile, derivationPath, seedPassphrase });
}
