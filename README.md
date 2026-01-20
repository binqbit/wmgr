WMGR - Solana & EVM Wallet Manager CLI
======================================

A minimal CLI to send SOL/USDC on Solana and ETH/ERC-20 (incl. BEP-20/PIP-20 compatible) on EVM networks. Built with a pluggable architecture so you can add new key resolvers (wallet name, KMS, etc.) and new assets later.

Quick Start
-----------

1) Install dependencies

   npm install

2) Run via Node directly

   node bin/wmgr.js --help

3) (Optional) Install globally

   npm link
   wmgr --help

Commands
--------

Solana
- Send SOL  
  `wmgr send:sol --to <RECIPIENT_PUBKEY> --amount <SOL> --keyfile <PATH_TO_KEY> [--cluster devnet|testnet|mainnet-beta] [--rpc <URL>]`

  Example:  
  `wmgr send:sol --to 9x...abc --amount 0.01 --keyfile C:\keys\id.json --cluster devnet`

- Send USDC (SPL Token)  
  `wmgr send:usdc --to <RECIPIENT_PUBKEY> --amount <USDC> --keyfile <PATH_TO_KEY> [--cluster devnet|testnet|mainnet-beta] [--rpc <URL>]`

  Example:  
  `wmgr send:usdc --to 9x...abc --amount 1.5 --keyfile C:\keys\id.json --cluster devnet`

- Check Balance  
  `wmgr balance --address <PUBKEY> [--cluster devnet|testnet|mainnet-beta]`  
  Or resolve from a wallet instead of an address:  
  `wmgr balance --keyfile C:\keys\id.json --cluster devnet`  
  `wmgr balance --seed "abandon ... art" --cluster devnet`  
  `wmgr balance --svpi [--svpi-name name] [--svpi-file path] --cluster devnet`

EVM (Ethereum-compatible)
- Send native ETH (or the network’s native coin)  
  `wmgr send:eth --to 0xRECIPIENT --amount <ETH> --privkey 0x... [--network mainnet|sepolia|polygon|bsc|avalanche|optimism|arbitrum|holesky|polygon_amoy|bsc_testnet|avalanche_fuji] [--rpc <URL>] [--gas-price <gwei>] [--gas-limit <num>]`

- Send ERC-20 / BEP-20 / PIP-20 token  
  `wmgr send:erc20 --token 0xTOKEN --to 0xRECIPIENT --amount <TOKENS> --privkey 0x... [--decimals <num>] [--network ...] [--rpc <URL>] [--gas-price <gwei>] [--gas-limit <num>]`

Notes:
- Default EVM derivation path is `m/44'/60'/0'/0/0`.
- You can swap `--privkey` for `--privkey-file`, `--seed`, or `--svpi` (mnemonic) just like Solana flows.
- Public RPC defaults are provided; override with `--rpc` for production-grade endpoints.

SVPI Wallet Interface
---------------------

Use `--svpi` to fetch a mnemonic from your external Wallet Manager via its CLI. The tool now talks to SVPI in JSON mode:

  svpi --mode=json get <name> --password=<password> [--file=<path>]

The CLI expects the `svpi.response.v1` envelope and reads `result.data` as the mnemonic.

Interactive flow (works for both Solana and EVM):

  wmgr send:sol --to <RECIPIENT_PUBKEY> --amount 0.01 --svpi --cluster devnet  
  wmgr send:eth --to 0xRECIPIENT --amount 0.01 --svpi --network sepolia

You will be prompted for:
- SVPI wallet name
- SVPI password (input hidden; not echoed)

Options:
- `--svpi-name <name>`: skip the interactive name prompt
- `--svpi-file <path>`: run `svpi` in file mode by passing `--file=<path>`
- `--path`, `--seed-passphrase`: applied to the derived mnemonic

Seed-based Key (BIP39)
----------------------

Instead of `--keyfile`/`--privkey`, you can provide a BIP39 seed phrase.

Solana example:  
  `wmgr send:sol --to <RECIPIENT_PUBKEY> --amount 0.01 --seed "abandon ..." --mnemo trustwallet --cluster devnet`

EVM example (default path `m/44'/60'/0'/0/0`):  
  `wmgr send:eth --to 0xRECIPIENT --amount 0.01 --seed "abandon ..." --network sepolia`

Notes:
- Solana: `--seed` and `--keyfile` are mutually exclusive. Default mnemonic profile is `trustwallet` (`m/44'/501'/0'/0'`).
- EVM: use `--path` to change the derivation; `--seed-passphrase` is supported for both Solana and EVM.

Mnemonic Profiles (Solana)
--------------------------

Choose a preset with `--mnemo <name>` (overridden by explicit `--path`):
- trustwallet: `m/44'/501'/0'/0'` (default)
- phantom: `m/44'/501'/0'`
- solflare: `m/44'/501'/0'`
- solana_cli: `m/44'/501'/0'/0'`

Key File Formats
----------------

- Solana: standard Solana CLI keypair JSON (array of 64 numbers). Generate with `solana-keygen new -o id.json`.
- EVM: plain hex private key (with or without `0x`), either passed directly via `--privkey` or stored in a file for `--privkey-file`.

Architecture
------------

- Key Resolvers
  - `src/keys/index.js`: Solana key resolver (file-based) and BIP39 derivation.
  - `src/keys/evm.js`: EVM wallet resolution from privkey / file / BIP39 / SVPI.

- Services
  - `src/services/sol.js`: native SOL transfers.
  - `src/services/spl.js`: SPL token transfers.
  - `src/services/evm.js`: native ETH and ERC-20/PIP-20/BEP-20 transfers.

- Config
  - `src/config/clusters.js`: Solana cluster RPC URLs and USDC mint addresses.
  - `src/config/evmNetworks.js`: EVM network presets (RPC + chain IDs).

- CLI
  - `src/cli.js`: commander-based CLI, with Solana and EVM subcommands.

Notes
-----

- Default Solana cluster is `mainnet-beta` (override with `--cluster` or `--rpc`).
- Default EVM network is `mainnet`; override with `--network` or `--rpc`.
- USDC decimals are inferred from the mint; ERC-20 decimals are fetched unless `--decimals` is provided.
- The CLI will create associated token accounts for Solana transfers when needed.
