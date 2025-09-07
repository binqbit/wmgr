import { Command } from 'commander';
import { resolveKeypair, keypairFromMnemonic } from './keys/index.js';
import { keypairFromSvpi } from './keys/svpi.js';
import { prompt, promptHidden } from './utils/prompt.js';
import { createConnection } from './solana/connection.js';
import { transferSol } from './services/sol.js';
import { transferSplToken } from './services/spl.js';
import { getBalances } from './services/balance.js';
import { getClusterConfig, getUsdcMintForCluster } from './config/clusters.js';
import { getMnemonicProfile, listMnemonicProfiles } from './config/mnemonics.js';
import { PublicKey } from '@solana/web3.js';

export async function main(argv = process.argv) {
  const program = new Command();
  program
    .name('wmgr')
    .description('Wallet Manager CLI for Solana transfers (SOL and USDC)')
    .version('1.0.0');

  const commonOptions = (cmd) =>
    cmd
      .requiredOption('--to <address>', 'recipient wallet address (base58)')
      .requiredOption('--amount <amount>', 'amount to send, e.g. 0.01 or 5')
      .option('--keyfile <pathOrRef>', 'path to keypair file (Solana CLI format). Future: wallet name')
      .option('--seed <mnemonic>', 'BIP39 mnemonic to derive key (mutually exclusive with --keyfile)')
      .option('--path <derivationPath>', "BIP44 derivation path for --seed")
      .option('--mnemo <name>', `mnemonic derivation profile (${listMnemonicProfiles().join('|')})`, 'trustwallet')
      .option('--seed-passphrase <pass>', 'BIP39 passphrase for --seed')
      .option('--svpi', 'Use SVPI interface to fetch mnemonic (prompts for name & password)')
      .option('--svpi-file <path>', 'SVPI file mode: pass --file=<path> to svpi')
      .option('--svpi-name <name>', 'SVPI wallet name (skip interactive name prompt)')
      .option('--cluster <name>', 'cluster: mainnet-beta | devnet | testnet', 'mainnet-beta')
      .option('--rpc <url>', 'custom RPC URL (overrides cluster default)')
      .option('--confirm <level>', 'commitment: processed | confirmed | finalized', 'confirmed');

  program
    .command('send:sol')
    .description('Send native SOL to a recipient')
    .action(async (opts) => handleSend('sol', opts))
    .configureOutput({
      outputError: (str, write) => write(`Error: ${str}`)
    });

  program
    .command('send:usdc')
    .description('Send USDC (SPL Token) to a recipient')
    .action(async (opts) => handleSend('usdc', opts))
    .configureOutput({
      outputError: (str, write) => write(`Error: ${str}`)
    });

  program
    .command('balance')
    .description('Show SOL and USDC balances for an address or resolved wallet')
    .option('--address <address>', 'public key to query (base58). If omitted, uses --keyfile/--seed/--svpi')
    .option('--keyfile <pathOrRef>', 'path to keypair file (Solana CLI format). Future: wallet name')
    .option('--seed <mnemonic>', 'BIP39 mnemonic to derive key (mutually exclusive with --keyfile)')
    .option('--path <derivationPath>', "BIP44 derivation path for --seed")
    .option('--mnemo <name>', `mnemonic derivation profile (${listMnemonicProfiles().join('|')})`, 'trustwallet')
    .option('--seed-passphrase <pass>', 'BIP39 passphrase for --seed')
    .option('--svpi', 'Use SVPI interface to fetch mnemonic (prompts for name & password)')
    .option('--svpi-file <path>', 'SVPI file mode: pass --file=<path> to svpi')
    .option('--svpi-name <name>', 'SVPI wallet name (skip interactive name prompt)')
    .option('--cluster <name>', 'cluster: mainnet-beta | devnet | testnet', 'mainnet-beta')
    .option('--rpc <url>', 'custom RPC URL (overrides cluster default)')
    .option('--confirm <level>', 'commitment: processed | confirmed | finalized', 'confirmed')
    .action(async (opts) => handleBalance(opts))
    .configureOutput({
      outputError: (str, write) => write(`Error: ${str}`)
    });

  // Attach common options only to transfer commands
  const sendSolCmd = program.commands.find(c => c.name() === 'send:sol');
  if (sendSolCmd) commonOptions(sendSolCmd);
  const sendUsdcCmd = program.commands.find(c => c.name() === 'send:usdc');
  if (sendUsdcCmd) commonOptions(sendUsdcCmd);

  await program.parseAsync(argv);
}

async function handleSend(kind, opts) {
  try {
    const { to, amount } = opts;
    const { keyfile, seed, path: pathOpt, mnemo, seedPassphrase, svpi, svpiFile, svpiName, cluster, rpc, confirm } = opts;
    const profile = getMnemonicProfile(mnemo);
    const derivationPath = pathOpt || profile.path;
    let keypair;
    if (svpi) {
      const name = svpiName || await prompt('SVPI wallet name: ');
      const password = await promptHidden('SVPI password: ');
      keypair = await keypairFromSvpi({ name, password, filePath: svpiFile, derivationPath, seedPassphrase });
    } else {
      if (!keyfile && !seed) {
        throw new Error('Provide either --keyfile, --seed, or use --svpi');
      }
      if ((keyfile && seed)) {
        throw new Error('Use only one of --keyfile, --seed, or --svpi');
      }
      keypair = keyfile
        ? await resolveKeypair(keyfile)
        : keypairFromMnemonic(seed, derivationPath, seedPassphrase || '');
    }
    const { url: rpcUrl } = getClusterConfig(cluster, rpc);
    const connection = createConnection(rpcUrl, confirm);

    if (kind === 'sol') {
      const sig = await transferSol({ connection, from: keypair, to, amount });
      console.log(`SUCCESS: SOL sent. Signature: ${sig}`);
    } else if (kind === 'usdc') {
      const mint = getUsdcMintForCluster(cluster);
      const sig = await transferSplToken({ connection, from: keypair, to, amount, mint });
      console.log(`SUCCESS: USDC sent. Signature: ${sig}`);
    } else {
      console.error('Unknown asset kind');
      process.exit(2);
    }
  } catch (err) {
    console.error(`Failed: ${err?.message || err}`);
    if (err?.stack) console.error(err.stack);
    process.exit(1);
  }
}

async function handleBalance(opts) {
  try {
    const { address, keyfile, seed, path: pathOpt, mnemo, seedPassphrase, svpi, svpiFile, svpiName, cluster, rpc, confirm } = opts;
    let ownerPk;
    if (address) {
      ownerPk = new PublicKey(address);
    } else {
      const profile = getMnemonicProfile(mnemo);
      const derivationPath = pathOpt || profile.path;
      let keypair;
      if (svpi) {
        const name = svpiName || await prompt('SVPI wallet name: ');
        const password = await promptHidden('SVPI password: ');
        keypair = await keypairFromSvpi({ name, password, filePath: svpiFile, derivationPath, seedPassphrase });
      } else {
        if (!keyfile && !seed) {
          throw new Error('Provide either --address, or one of --keyfile/--seed/--svpi');
        }
        if ((keyfile && seed)) {
          throw new Error('Use only one of --keyfile or --seed');
        }
        keypair = keyfile
          ? await resolveKeypair(keyfile)
          : keypairFromMnemonic(seed, derivationPath, seedPassphrase || '');
      }
      ownerPk = keypair.publicKey;
    }

    const { url: rpcUrl } = getClusterConfig(cluster, rpc);
    const connection = createConnection(rpcUrl, confirm);
    const mint = getUsdcMintForCluster(cluster);
    const res = await getBalances({ connection, owner: ownerPk, mint });
    console.log(`Address: ${res.address}`);
    console.log(`SOL: ${res.sol}`);
    console.log(`USDC: ${res.usdc}`);
  } catch (err) {
    console.error(`Failed: ${err?.message || err}`);
    if (err?.stack) console.error(err.stack);
    process.exit(1);
  }
}
