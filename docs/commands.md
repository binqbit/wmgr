# Commands

This is a command reference for `wmgr`. For a quick overview, see the top-level `README.md`.

## Solana wallets

```sh
wmgr wallet new
wmgr wallet new --format mnemonic
wmgr wallet vanity --prefix Ab --suffix xy
wmgr wallet import --from wallet.json
wmgr wallet export --svpi-name trading
wmgr wallet recover --from recovery.txt --no-svpi
```

Saving defaults to SVPI and prompts for a name and password. An empty response skips it; `--no-svpi`
saves locally. Skipping or a failed save creates `<public-key>.json` — a standard Solana
64-byte keypair. Mnemonics also get a `.txt` with the phrase, path, and address; public-only
wallets get only address text. Export uses the same formats. `--out-dir DIR` selects an
existing directory instead of the current one. Existing files and SVPI entries are not replaced.

Mnemonics default to 24 words and `m/44'/501'/0'`. Use `--passphrase` for BIP39 passphrase
input during generation or recovery/import/export; it is never saved.

Vanity accepts a prefix, suffix, or both. It uses GPU when available, otherwise CPU.
Matching is case sensitive unless `--ignore-case` is set.

Import detects the format automatically. For 32-byte values, hex/JSON means a private seed
and Base58 means a public key. For a bare phrase, use `--path` if its derivation differs.

Other options: `wmgr wallet <command> --help`. See [security](security.md#offline-wallet-generation)
for backup handling.

## Solana programs

These commands manage upgradeable loader-v3 programs.

```sh
wmgr program deploy
wmgr program deploy target/deploy/app.so --program app --payer deployer --authority admin
wmgr program upgrade target/deploy/app.so --program app --payer deployer --authority admin
wmgr program show --program app
wmgr program set-authority --program app --payer deployer --authority admin --new-authority next-admin
```

Missing values are prompted: program file, program wallet, payer, then authority. Wallet names
refer to SVPI records. The payer prompt uses the configured wallet as its default; authority
defaults to the payer. Passwords are prompted once per distinct record, without echo.

Deploy requires the program's private key; upgrade and show also accept a public-only record.
Upgrade checks the existing authority and extends program storage when needed. Authority
transfer requires both the current and new authority to sign. Keys stay in memory.

Program commands always use SVPI. RPC and SVPI connection defaults come from `.wmgr`;
`--cluster`, `--rpc`, `--svpi-file`, and `--svpi-cmd` override them. Use
`--compute-unit-price` to set a deployment priority fee in micro-lamports per compute unit.
In a terminal, Solana's progress indicator shows upload and transaction confirmation progress.

The deployment buffer address is printed before upload. After failure, retry with
`--buffer ADDRESS`, or reclaim its SOL with
`wmgr program close-buffer --buffer ADDRESS --payer deployer --authority admin`.
The refund goes to the payer. If a retry requires more program space, omit `--buffer`
and reclaim the old buffer separately. Other options: `wmgr program <command> --help`.

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
wmgr send <AMOUNT> sol  <TO> [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
wmgr send <AMOUNT> usdc <TO> [--keyfile <PATH> | --seed <MNEMONIC> | --svpi] \
  [--cluster <name>] [--rpc <url>] [--commitment <processed|confirmed|finalized>]
```

For example, send 1 USDC:

```sh
wmgr send 1 usdc <TO>
```

EVM:

```sh
wmgr send <AMOUNT> eth   <TO> [--privkey <HEX> | --privkey-file <PATH> | --seed <MNEMONIC> | --svpi] \
  [--network <name>] [--rpc <url>] [--gas-price <gwei>] [--gas-limit <num>]
wmgr send <AMOUNT> erc20 <TOKEN> <TO> [--decimals <num>] \
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

Examples:

```sh
wmgr buy 1 usdc
wmgr sell 1 sol
```

Notes:

- Trading is currently integrated for the Raydium SOL/USDC AMM pool.
- `buy` treats the amount as the exact output to receive; `sell` treats it as the exact input to
  spend.
- `--slippage` is a percent value (default `0.1` = 0.1%).
