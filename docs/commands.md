# Commands

This is a command reference for `wmgr`. For a quick overview, see the top-level `README.md`.

## Interactive mode (REPL)

Start REPL:

```sh
wmgr
```

Or explicitly:

```sh
wmgr repl
```

REPL helpers:

- `help` / `?` — show CLI help
- `exit` / `quit` / `q` — exit
- `clear` / `cls` — clear screen and history

## Config (`.wmgr`)

Show current config:

```sh
wmgr config show
```

Set defaults:

```sh
wmgr config set --svpi --svpi-name <name>
wmgr config set --cluster mainnet-beta --commitment confirmed --rpc <solana-rpc>
wmgr config set --network mainnet --rpc <evm-rpc> --gas-price <gwei> --gas-limit <num>
```

Reset (remove `.wmgr` in current directory):

```sh
wmgr config reset
```

## Self hash

Print SHA256 hashes for the current executable and config files:

```sh
wmgr self-hash
```

Alias:

```sh
wmgr hash
```

Output is grouped:

- `wmgr:` — always printed
- `svpi:` — printed only when SVPI mode is enabled in `.wmgr`

## Balance

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

If `ADDRESS` is omitted, `wmgr` resolves the wallet from the provided key source and uses its
address/public key.

## Send

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

## Price

Show SOL/USDC price from the Raydium pool:

```sh
wmgr price <sol|usdc> [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
```

## Buy / Sell (Raydium SOL/USDC)

```sh
wmgr buy <AMOUNT> <sol|usdc> [--slippage <percent>] \
  [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]

wmgr sell <AMOUNT> <sol|usdc> [--slippage <percent>] \
  [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
```

Notes:

- Trading is currently integrated for the Raydium SOL/USDC AMM pool.
- `--slippage` is a percent value (default `0.1` = 0.1%).
