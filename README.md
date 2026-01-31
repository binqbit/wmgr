# wmgr - Wallet Manager CLI

## Project Description

`wmgr` is a compact non-custodial CLI for Solana and EVM wallets:

- check balances
- send tokens
- trade SOL/USDC on Raydium
- optional interactive REPL + saved defaults (`.wmgr`)

For key management, `wmgr` can integrate with **SVPI** to fetch mnemonics/private keys from an
encrypted vault at runtime instead of storing secrets in files or env vars.

## Documentation

Technical documentation lives in `docs/`:

- [docs/security.md](docs/security.md) — security model, threat assumptions, operational tips
- [docs/config.md](docs/config.md) — `.wmgr` defaults, `config set/show/reset`, flag resolution
- [docs/commands.md](docs/commands.md) — command reference and examples
- [docs/architecture.md](docs/architecture.md) — high-level architecture and data flows

## Quick Start

```sh
cargo build
cargo run -- --help
```

Run interactive mode:

```sh
wmgr
```

## Build

For a production build you can use:

- Linux/macOS: `build.sh` (builds `--release`, copies `wmgr` to `./bin/`)

## Related repositories

- [svpi](https://github.com/binqbit/svpi) — Secure Vault Personal Information (encrypted vault used
  as a key source for `wmgr`; see SVPI docs for its architecture and security model)
