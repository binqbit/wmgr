WMGR — Solana Wallet Manager CLI
================================

A minimal, extensible CLI to send SOL and USDC on the Solana blockchain. Designed with a pluggable architecture so you can add new key resolvers (e.g., by wallet name, KMS) and new assets later.

Quick Start
----------

1) Install dependencies

   npm install

2) Run via Node directly

   node bin/wmgr.js --help

3) (Optional) Install the command globally in your shell

   npm link
   wmgr --help

Commands
--------

- Send SOL

  wmgr send:sol --to <RECIPIENT_PUBKEY> --amount <SOL> --keyfile <PATH_TO_KEY> [--cluster devnet|testnet|mainnet-beta] [--rpc <URL>]

  Example:

  wmgr send:sol --to 9x...abc --amount 0.01 --keyfile C:\\keys\\id.json --cluster devnet

- Send USDC (SPL Token)

  wmgr send:usdc --to <RECIPIENT_PUBKEY> --amount <USDC> --keyfile <PATH_TO_KEY> [--cluster devnet|testnet|mainnet-beta] [--rpc <URL>]

  Example:

  wmgr send:usdc --to 9x...abc --amount 1.5 --keyfile C:\\keys\\id.json --cluster devnet

- Check Balance

  wmgr balance --address <PUBKEY> [--cluster devnet|testnet|mainnet-beta]

  Or resolve from a wallet instead of an address:

  wmgr balance --keyfile C:\\keys\\id.json --cluster devnet
  wmgr balance --seed "abandon ... art" --cluster devnet
  wmgr balance --svpi [--svpi-name name] [--svpi-file path] --cluster devnet

  Prints a simple list with SOL and USDC balances for the cluster.

SVPI Wallet Interface
---------------------

Use `--svpi` to fetch a mnemonic from your external Wallet Manager via its CLI. The tool runs:

  svpi get <name> --password=<password>

It parses a line like `Data: <mnemonic>` from the command output and uses it to derive the key.

Interactive flow:

  wmgr send:sol --to <RECIPIENT_PUBKEY> --amount 0.01 --svpi --cluster devnet

You will be prompted for:
- SVPI wallet name
- SVPI password (input hidden; not echoed)

Options:
- `--svpi-name <name>`: skip the interactive name prompt
- `--svpi-file <path>`: run `svpi` in file mode by passing `--file=<path>`
- `--path`, `--seed-passphrase`: applied to the derived mnemonic

Seed-based Key (BIP39)
----------------------

Instead of `--keyfile`, you can provide a BIP39 seed phrase:

  wmgr send:sol --to <RECIPIENT_PUBKEY> --amount 0.01 --seed "abandon abandon ... art" --mnemo trustwallet --cluster devnet
  # override the path explicitly if needed
  wmgr send:sol --to <RECIPIENT_PUBKEY> --amount 0.01 --seed "abandon ..." --path "m/44'/501'/0'" --cluster devnet

Notes:
- `--seed` and `--keyfile` are mutually exclusive (use one).
- Default mnemonic profile is `trustwallet` which uses path `m/44'/501'/0'/0'`.
- `--seed-passphrase` is optional BIP39 passphrase.

Mnemonic Profiles
-----------------

Choose a preset with `--mnemo <name>` (overridden by explicit `--path`):
- trustwallet: `m/44'/501'/0'/0'` (default)
- phantom: `m/44'/501'/0'`
- solflare: `m/44'/501'/0'`
- solana_cli: `m/44'/501'/0'/0'`

Key File Format
---------------

- Uses the standard Solana CLI keypair JSON (an array of 64 numbers). Generate with:

  solana-keygen new -o id.json

Architecture
------------

- Key Resolver
  - src/keys/index.js defines a KeyResolver interface and a FileKeyResolver.
  - Future: add resolvers by wallet name, external KMS, etc., then chain them.

- Services
  - src/services/sol.js handles native SOL transfers.
  - src/services/spl.js handles SPL token transfers using associated token accounts.

- Config
  - src/config/clusters.js centralizes cluster RPC URLs and USDC mint addresses.

- CLI
  - src/cli.js based on commander, with shared options and subcommands.

Notes
-----

- Default cluster is devnet. Override with --cluster or --rpc.
- USDC decimals are inferred from the on-chain mint; mainnet mint is EPjF..., devnet uses 4zMMC....
- The CLI will create associated token accounts for sender and recipient if missing.
