# Architecture (high level)

`wmgr` is a single binary that exposes a Clap-based CLI and an optional interactive REPL.

This document is a high-level overview of how pieces fit together.

## Components

- **CLI parsing:** `clap` derives the command tree (`wmgr --help`).
- **REPL mode:** when no command is provided (or `wmgr repl`), `wmgr` reads a line, splits it with
  `shlex`, and reuses the same Clap parser as the non-interactive CLI.
- **Defaults/config layer:** `.wmgr` is loaded from the current working directory and used to apply
  defaults when flags are not provided.
- **Key resolution:**
  - **Solana:** keypair file (`--keyfile`), BIP39 seed (`--seed`), or SVPI (`--svpi`).
  - **EVM:** private key (`--privkey`/`--privkey-file`), BIP39 seed (`--seed`), or SVPI (`--svpi`).
- **Network clients:**
  - **Solana:** `solana-client` RPC for balance queries and transactions.
  - **EVM:** `ethers` provider for balance queries and transactions.
  - **Raydium:** `raydium-amm-swap` for SOL/USDC quotes and swap instruction building.

## Config and defaults

At startup `wmgr` tries to load `.wmgr` from the current directory. If it does not exist (or cannot
be deserialized), defaults are used.

Defaults are applied conservatively:

- CLI flags always take priority.
- Config is only used when a flag is not provided.
- Behavior is not “auto-switched” based on input shape (example: EVM balance is enabled only when
  `--network` is provided).

## SVPI integration

When `--svpi` is enabled, `wmgr` executes an external `svpi` binary in JSON mode to fetch a secret
value by name:

- secret can be a mnemonic or a hex private key (depending on target chain)
- `wmgr` derives the chain-specific signer in memory

The SVPI executable path can be overridden via `--svpi_cmd` or `.wmgr` defaults.

## Integrity check (self-hash)

`wmgr self-hash` computes SHA256 hashes for the `wmgr` binary and `.wmgr` config file in the
current directory; when SVPI mode is enabled it also hashes the `svpi` executable and `.svpi`
config file.
