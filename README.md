# wmgr - Wallet Manager CLI

Compact CLI for Solana and EVM wallets: balance, send, and Raydium SOL/USDC trading.

## Quick Start

```sh
cargo build
cargo run -- --help
```

## Commands (ordered)

### 1) Balance

Solana (default):

```sh
wmgr balance [ADDRESS] [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
```

EVM (enable with `--network`):

```sh
wmgr balance [ADDRESS] --network <name> \
  [--privkey <HEX> | --privkey-file <PATH> | --seed <MNEMONIC> | --svpi] \
  [--rpc <url>]
```

### 2) Send

Solana:

```sh
wmgr send sol  <TO> <AMOUNT> [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
wmgr send usdc <TO> <AMOUNT> [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
```

EVM:

```sh
wmgr send eth   <TO> <AMOUNT> [--privkey <HEX> | --privkey-file <PATH> | --seed <MNEMONIC> | --svpi] \
  [--network <name>] [--rpc <url>] [--gas-price <gwei>] [--gas-limit <num>]
wmgr send erc20 <TOKEN> <TO> <AMOUNT> [--decimals <num>] \
  [--privkey <HEX> | --privkey-file <PATH> | --seed <MNEMONIC> | --svpi] \
  [--network <name>] [--rpc <url>] [--gas-price <gwei>] [--gas-limit <num>]
```

### 3) Price

```sh
wmgr price <sol|usdc> [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
```

### 4) Buy

```sh
wmgr buy <AMOUNT> <sol|usdc> [--slippage <percent>] \
  [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
```

### 5) Sell

```sh
wmgr sell <AMOUNT> <sol|usdc> [--slippage <percent>] \
  [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
```

## Key Sources

Solana:

- `--keyfile <PATH>` Solana CLI JSON keypair (64-byte array)
- `--seed <MNEMONIC>` BIP39 seed phrase
- `--svpi` Fetch mnemonic or hex private key from SVPI

EVM:

- `--privkey <HEX>` Private key (with or without 0x)
- `--privkey-file <PATH>` File containing the private key
- `--seed <MNEMONIC>` BIP39 seed phrase
- `--svpi` Fetch mnemonic or hex private key from SVPI

## SVPI (JSON mode)

```sh
svpi --mode=json get <name> --password=<password> [--file=<path>]
```

Use `--svpi_cmd <path>` to point to a custom binary. The CLI expects the
`svpi.response.v1` envelope and reads `result.data` as the mnemonic or hex
private key (32 bytes for EVM, 32 or 64 bytes for Solana).

## Notes

- Default Solana cluster: `mainnet-beta`
- Default EVM network: `mainnet`
- Balance uses Solana unless `--network` is set
- `--slippage` is a percent value (default `0.1` = 0.1%)
- EVM networks: `mainnet`, `sepolia`, `holesky`, `polygon`, `polygon_amoy`,
  `bsc`, `bsc_testnet`, `avalanche`, `avalanche_fuji`, `optimism`, `arbitrum`
- Buy/Sell uses the Raydium SOL/USDC AMM pool (quotes via Raydium HTTP API)
