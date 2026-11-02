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
- **Local key handling:** generation and storage are separate from network clients.
  CPU and GPU execution share validation and result checks.
- **Network clients:**
  - **Solana:** `solana-client` RPC for balance queries and transactions.
    Program deployment uses the official Solana deployment engine with signers resolved in memory.
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

`wmgr` reads and writes vault records through SVPI 8.1 or later in JSON mode,
validating the `svpi.response.v2` schema, operation status, and process exit status:

- secrets retain their key type; Solana mnemonic records also retain their derivation path
- `wmgr` derives the chain-specific signer in memory
- reads use `result.record` metadata and `result.data`; lists use `result.records`
- saves explicitly use plain-text encoding and automatic encryption selection: a matching
  named key when available, otherwise the supplied password directly
- an encrypted save is verified by reading the wallet back

The SVPI executable path can be overridden via `--svpi-cmd` (`--svpi_cmd`) or `.wmgr`
defaults. The child process inherits the working directory so SVPI can select its `.svpi`
default profile and file, serial-device, or HTTP storage. `--svpi-file` selects a file instead.
SVPI manages HTTP sessions and tokens; `wmgr` does not store them in `.wmgr`.

Existing names are refused before saving. Local storage retains a race between that check
and its unconditional write; HTTP storage creates records atomically when no revision is
supplied. Failed or unverified saves produce a local backup. If a write's outcome is unknown,
the backup can coexist with an accepted SVPI write; inspect the record before retrying.

## Integrity check (self-hash)

`wmgr self-hash` computes SHA256 hashes for the `wmgr` binary and `.wmgr` config file in the
current directory; when SVPI mode is enabled it also hashes the `svpi` executable and `.svpi`
config file.
