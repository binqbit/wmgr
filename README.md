# wmgr - Wallet Manager CLI

## Project Description

`wmgr` is a compact non-custodial CLI for Solana and EVM wallets:

- check balances
- send tokens
- trade SOL/USDC on Raydium
- generate Solana keypairs or BIP39 recovery phrases offline
- find Solana addresses with a chosen prefix and/or suffix using GPU or CPU
- import and export Solana wallets through SVPI
- deploy and upgrade Solana programs with signers from SVPI
- optional interactive REPL + saved defaults (`.wmgr`)

For key management, `wmgr` can integrate with **SVPI** to fetch mnemonics/private keys from an
encrypted vault at runtime instead of storing secrets in files or env vars.
This integration requires SVPI 8.1 or later with the `svpi.response.v2` JSON contract.
It uses SVPI's selected `.svpi` profile and storage from the current directory
(file, serial device, or HTTP provider); `--svpi-file` overrides the storage file.

## Documentation

Technical documentation lives in `docs/`:

- [docs/security.md](docs/security.md) — security model, threat assumptions, operational tips
- [docs/config.md](docs/config.md) — `.wmgr` defaults, `config set/show/reset`, flag resolution
- [docs/commands.md](docs/commands.md) — command reference and examples
- [docs/architecture.md](docs/architecture.md) — high-level architecture and data flows

Agent guidance: [wmgr skill](.agents/skills/wmgr/SKILL.md) covers command explanations,
wallet operations, shell/Python coordination, and Solana program management.

## Quick Start

```sh
cargo build
cargo run -- --help
```

Run interactive mode:

```sh
wmgr
```

Create a Solana wallet:

```sh
wmgr wallet new
wmgr wallet new --format mnemonic
wmgr wallet vanity --prefix Ab --suffix xy
```

Wallets save to SVPI by default, with local backup on skip or failure.
See [wallet commands](docs/commands.md#solana-wallets) for import, export, and recovery.

## Build

Install locally with the dependency versions recorded in `Cargo.lock`:

```sh
cargo install --path . --locked
```

For a production build you can use:

- Linux/macOS: `build.sh` (builds `--release`, copies `wmgr` to `./bin/`)

GPU support is included in every build. Vanity search automatically uses an available
OpenCL GPU, otherwise CPU. Only the GPU driver/runtime is needed; no separate build is required.

Run `cargo test --locked` for local tests. With SVPI 8.1 installed, run
`cargo test --locked --test svpi_cli -- --ignored` for real integration checks using
isolated temporary vaults. Set `SVPI_BIN` to select a specific executable.

## Related repositories

- [svpi](https://github.com/binqbit/svpi) — Secure Vault Personal Information (encrypted vault used
  as a key source for `wmgr`; see SVPI docs for its architecture and security model)
