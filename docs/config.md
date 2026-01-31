# Configuration (`.wmgr`)

`wmgr` can persist default launch mode and commonly used flags in a local `.wmgr` file (in the
current working directory).

This is intended for **defaults only** (RPCs, networks, slippage, SVPI name, etc.). Private keys,
mnemonics, and passwords are not stored.

## Commands

- `wmgr config show` — print current config values
- `wmgr config set [OPTIONS]` — update config values
- `wmgr config reset` — remove `.wmgr` and reset in-memory defaults

## What is stored

- **SVPI defaults**
  - `--svpi` mode (on/off)
  - `--svpi-name`
  - `--svpi-file`
  - `--svpi_cmd`
- **Solana defaults**
  - `--cluster`
  - `--rpc` (Solana RPC)
  - `--commitment`
  - `--slippage`
- **EVM defaults**
  - `--network`
  - `--rpc` (EVM RPC)
  - `--gas-price`
  - `--gas-limit`

## Defaults behavior

- Config values are applied only when the corresponding CLI flags are not provided.
- The core semantics of commands do not change. Example: `wmgr balance` is Solana unless
  `--network` is set.
- If SVPI mode is enabled in `.wmgr` and you do not provide any key source flags, `wmgr` will
  default to `--svpi`.

## RPC flag resolution for `config set`

`wmgr config set --rpc <URL>` updates:

- **Solana RPC only** when any Solana-related option is present (`--cluster`, `--commitment`,
  `--slippage`).
- **EVM RPC only** when any EVM-related option is present (`--network`, `--gas-price`,
  `--gas-limit`).
- **Both** Solana and EVM RPCs when neither side is specified (or when both sides are specified).

Examples:

```sh
# Set both RPCs
wmgr config set --rpc https://example-rpc

# Set Solana RPC only
wmgr config set --cluster mainnet-beta --rpc https://solana-rpc

# Set EVM RPC only
wmgr config set --network mainnet --rpc https://eth-rpc
```

## File format and compatibility

`.wmgr` is a small binary file (Borsh). It is not meant to be edited manually.

`wmgr` does not implement config versioning/migrations. If `.wmgr` cannot be deserialized (e.g.
after schema changes), it is treated as “not found” and defaults are used. Re-run `wmgr config set`
to recreate it.
