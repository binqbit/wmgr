# WMGR (Rust)

Rust rewrite of the WMGR CLI for Solana and EVM networks with a cleaner module layout and more ergonomic commands.

## Quick Start

1. Build

   cargo build

2. Run

   cargo run -- --help

## Commands

Solana

- Send SOL:
  wmgr send sol <TO> <AMOUNT> [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]

- Send USDC (SPL):
  wmgr send usdc <TO> <AMOUNT> [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]

- Balance:
  wmgr balance [ADDRESS] [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]

EVM

- Send native token:
  wmgr send eth <TO> <AMOUNT> [--privkey <HEX> | --privkey-file <PATH> | --seed <MNEMONIC> | --svpi] [--network <name>] [--rpc <url>] [--gas-price <gwei>] [--gas-limit <num>]

- Send ERC-20 token:
  wmgr send erc20 <TOKEN> <TO> <AMOUNT> [--decimals <num>] [--privkey <HEX> | --privkey-file <PATH> | --seed <MNEMONIC> | --svpi] [--network <name>] [--rpc <url>] [--gas-price <gwei>] [--gas-limit <num>]

## Key Sources

Solana:

- --keyfile <PATH> Solana CLI JSON keypair file (array of 64 numbers)
- --seed <MNEMONIC> BIP39 seed phrase (use --path or --mnemo to select derivation)
- --svpi Fetch mnemonic from SVPI (prompts for name and password)

EVM:

- --privkey <HEX> Private key (with or without 0x prefix)
- --privkey-file <PATH> File containing the private key
- --seed <MNEMONIC> BIP39 seed phrase (use --path to select derivation)
- --svpi Fetch mnemonic from SVPI (prompts for name and password)

## SVPI Integration

SVPI is accessed in JSON mode (override binary via `--svpi_cmd <path>` if needed):

  svpi --mode=json get <name> --password=<password> [--file=<path>]

The CLI expects the svpi.response.v1 envelope and reads result.data as the mnemonic.

## Notes

- Default Solana cluster is mainnet-beta.
- Default EVM network is mainnet.
- Use --rpc to override public RPC endpoints.
