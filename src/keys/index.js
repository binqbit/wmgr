import { stat, readFile } from 'fs/promises';
import path from 'path';
import { Keypair } from '@solana/web3.js';
import { mnemonicToSeedSync, validateMnemonic } from 'bip39';
import { derivePath } from 'ed25519-hd-key';

export class KeyResolver {
  // eslint-disable-next-line no-unused-vars
  async resolve(ref) {
    throw new Error('Not implemented');
  }
}

export class FileKeyResolver extends KeyResolver {
  async resolve(ref) {
    const maybePath = path.resolve(process.cwd(), ref);
    let st;
    try { st = await stat(maybePath); } catch (_) {}
    if (!st || !st.isFile()) {
      throw new Error('FileKeyResolver: path does not exist or is not a file');
    }
    const buf = await readFile(maybePath, 'utf8');
    // Expect Solana CLI keypair file (JSON array of 64 bytes)
    try {
      const arr = JSON.parse(buf);
      if (!Array.isArray(arr)) {
        throw new Error('Key file must be JSON array of numbers');
      }
      const secret = Uint8Array.from(arr);
      const kp = Keypair.fromSecretKey(secret);
      return kp;
    } catch (e) {
      throw new Error(`Failed to parse keypair file: ${e.message}`);
    }
  }
}

// Future: chain multiple resolvers (e.g., by name, KMS, etc.)
const resolvers = [new FileKeyResolver()];

export async function resolveKeypair(ref) {
  const errors = [];
  for (const r of resolvers) {
    try { return await r.resolve(ref); } catch (e) { errors.push(e.message); }
  }
  throw new Error(`Unable to resolve wallet. Provided ref is neither a valid file nor supported name. Details: ${errors.join('; ')}`);
}

export function keypairFromMnemonic(mnemonic, derivationPath = "m/44'/501'/0'/0'", passphrase = '') {
  if (!validateMnemonic(mnemonic)) {
    throw new Error('Invalid BIP39 mnemonic');
  }
  const seed = mnemonicToSeedSync(mnemonic, passphrase); // 64 bytes
  const { key } = derivePath(derivationPath, seed.toString('hex'));
  if (!key || key.length !== 32) {
    throw new Error('Failed to derive ed25519 key from seed');
  }
  return Keypair.fromSeed(Buffer.from(key));
}
