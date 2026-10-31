# Commands and operational semantics

Use the chosen binary's subcommand help to check version differences. Examples
below use shell variables for public operational inputs; fill those from the
user's actual request. Passwords are omitted so native hidden prompts can work.

## Key sources and defaults

| Operation family | Key selection |
| --- | --- |
| General Solana balance/send/trade | One of `--svpi --svpi-name NAME`, `--keyfile PATH`, or `--seed PHRASE` |
| General EVM balance/send | One of `--svpi --svpi-name NAME`, `--privkey-file PATH`, `--privkey HEX`, or `--seed PHRASE` |
| Solana program management | SVPI role names; see [programs.md](programs.md) |
| Wallet creation/import/recovery | New SVPI name or explicit local output; see below |

Any general SVPI-specific option activates SVPI mode. Do not combine it with
another key source. Prefer SVPI or an already authorized private key file over
placing keys or mnemonics in command arguments.

SVPI options include `--svpi-name`, `--svpi-file`, `--svpi-cmd` (also
`--svpi_cmd`), and `--svpi-pass`. Missing names/passwords can prompt. A configured
SVPI default can be used when no explicit key source is supplied. Scripts should
identify their source records explicitly instead of inheriting an unrelated one.

For ordinary commands, `--path` controls mnemonic derivation and
`--seed-passphrase` supplies a BIP39 passphrase, defaulting to empty. Solana
also supports `--mnemo` profiles; the default `trustwallet` path is
`m/44'/501'/0'`. EVM defaults to `m/44'/60'/0'/0/0`. Preserve the known address,
path, and passphrase rather than deriving a different wallet by assumption.
EVM SVPI records accept a bare hex private key or mnemonic, not structured
Solana wallet records. General Solana commands do not have program commands'
automatic extra BIP39 prompting.

`wmgr config show` reads current-directory defaults. `config set` changes them;
prefer per-command flags when preparing a task-specific operation. A missing or
unreadable `.wmgr` can result in fallback defaults, not a validated configuration.
`self-hash` / `hash` prints executable/config hashes; compare with a trusted
expected value before calling that an integrity verification.

## Balance queries

```sh
wmgr balance "$solana_address" --cluster "$cluster" --rpc "$rpc"
wmgr balance --svpi --svpi-name "$record" --cluster "$cluster" --rpc "$rpc"
wmgr balance "$evm_address" --network "$network" --rpc "$evm_rpc"
```

An explicit address bypasses key loading. Solana reports SOL and the configured
USDC associated-token-account balance; a public-only SVPI record also works.
EVM reports only the native token, and requires `--network` even if an EVM
network is saved. Prefer an explicit address over decrypting an EVM signer to
discover its balance.

Current Solana balance code treats any USDC account-fetch error as zero and
uses six decimals if mint metadata fails. If a decision depends on verified
holdings, distinguish account-not-found from RPC failure with independent
read-only account/token queries. Do not turn a failed read into confirmed zero.

## Transfers and distributions

```sh
wmgr send "$amount" sol "$recipient" \
  --svpi --svpi-name "$sender" --cluster "$cluster" --rpc "$rpc"
wmgr send "$amount" usdc "$recipient" \
  --svpi --svpi-name "$sender" --cluster "$cluster" --rpc "$rpc"
wmgr send "$amount" eth "$recipient" \
  --svpi --svpi-name "$sender" --network "$network" --rpc "$evm_rpc"
wmgr send "$amount" erc20 "$token_contract" "$recipient" \
  --svpi --svpi-name "$sender" --network "$network" --rpc "$evm_rpc"
```

- Amount comes **before** the asset subcommand. Values are human decimal units:
  SOL has nine decimals, EVM native tokens eighteen, and tokens use their mint
  or contract decimals. Optional ERC20 `--decimals N` overrides metadata; use it
  only when the actual token precision is known.
- The Solana amount parser truncates extra fractional digits. Validate positive
  amounts and supported precision before calling it. Compute distributions with
  integer base units or decimal arithmetic, never binary floating point. Specify
  who receives rounding remainders instead of silently losing or overallocating them.
- `eth` means the selected EVM network's native token. `usdc` means the built-in
  Solana mint for that cluster; EVM USDC is `erc20` with its verified contract.
  This is not a generic SPL-token command. Localnet does not create token mints.
- For USDC, `recipient` is the wallet owner address, not its token account. The
  sender pays transaction fees and the rent for missing associated token accounts.
  Ordinary sends do not support a separate fee payer. Keep native currency for
  gas/fees and required account creation, including at intermediate wallets.
- EVM overrides are `--gas-price` in **gwei** and `--gas-limit` in gas units.
  There is no nonce option or sweep-all option. Serialize operations from the
  same signer unless nonce coordination is supplied by another supported tool.
- Send commands start without a final confirmation prompt. Validate the intended
  network, source identity, recipients, totals, and fee reserve before invoking.

Solana sends print a signature after confirmation at the chosen
`--commitment processed|confirmed|finalized` (default `confirmed`). Current EVM
sends print a transaction hash but discard the receipt's success status. Verify
receipt existence and successful status independently; for ERC20 also verify
the expected transfer result. An exit-zero `SUCCESS` line alone is insufficient.

Network failures may occur after submission, before a signature/hash is printed.
Treat that row as uncertain and reconcile chain history/receipts before resending.
Independent transfers are not an atomic batch. See [automation.md](automation.md).

## Wallet creation, import, recovery, and export

```sh
wmgr wallet new --svpi-name "$new_record" --out-dir "$private_backup_dir"
wmgr wallet new --format mnemonic --svpi-name "$new_record" --out-dir "$private_backup_dir"
wmgr wallet vanity --prefix Ab --suffix xy --svpi-name "$new_record" --out-dir "$private_backup_dir"
wmgr wallet import --from "$wallet_file" --svpi-name "$new_record" --out-dir "$private_backup_dir"
wmgr wallet recover --from "$recovery_file" --svpi-name "$new_record" --out-dir "$private_backup_dir"
wmgr wallet export --svpi-name "$record" --out-dir "$private_backup_dir"
```

`wallet` creates/imports/exports Solana material. It is not an EVM wallet creation
API. Generation is local; SVPI storage uses its configured file/device. The
output directory must already exist. New records are not named implicitly after
the configured default, and existing SVPI entries/files are not overwritten.

Saving defaults to SVPI. An empty name/password, or a failed SVPI save, falls back
to local plaintext backup and can still exit successfully. Set an existing private
`--out-dir` outside the repository even when intending to save to SVPI. Verify the
reported `SVPI entry:` or `File:`/`Recovery file:` outcome rather than just exit status.
Use `--no-svpi` only when local plaintext output is intended.

Local secret wallets use `<public-key>.json`; mnemonics also create a recovery
`.txt`; public-only material creates address text. Unix secret files use `0600`.
Do not include these files or their contents in reports, source control, or logs.
BIP39 passphrases are never saved and do not encrypt the backup keypair.

Mnemonic defaults: 24 words and `m/44'/501'/0'`. `--passphrase` prompts for the
BIP39 passphrase; generation confirms it. Import detects the format; for bare
mnemonics preserve the correct `--path`. A 32-byte hex/JSON value means a private
seed, while a 32-byte Base58 value means a public key. Do not guess the input type.

Vanity search accepts prefix, suffix, or both; matching is case-sensitive unless
`--ignore-case`. `--backend auto|cpu|gpu`, `--threads`, and `--max-attempts` control
the search. Auto uses a working OpenCL GPU or falls back to CPU. Use CPU when the
requested trust boundary excludes exposing candidate seeds to a GPU/driver.

## Raydium price and swaps

```sh
wmgr price sol --cluster "$cluster" --rpc "$rpc"
wmgr buy "$amount" sol --slippage "$slippage_percent" \
  --svpi --svpi-name "$trader" --cluster "$cluster" --rpc "$rpc"
wmgr sell "$amount" usdc --slippage "$slippage_percent" \
  --svpi --svpi-name "$trader" --cluster "$cluster" --rpc "$rpc"
```

Price needs no wallet; supplied key options are ignored. It is a pool reserve
ratio, not an executable quote. Price and swaps use one hardcoded mainnet
Raydium SOL/USDC pool and mints. Changing `--cluster` does not create a devnet
trading setup; a non-mainnet environment would need appropriate fixtures.

`buy 1 sol` requests exactly 1 SOL output and spends USDC up to the bound.
`sell 1 sol` spends exactly 1 SOL and receives USDC above the minimum.
Slippage is a percentage: `0.1` means 0.1%, not 10%; pin it explicitly in scripts.

Both swap commands simulate and ask `Continue? (y/N):` on standard input.
There is no `--yes`. Declining prints `Aborted.` and exits zero. Retain human
confirmation unless the user authorized automatic confirmation of that exact
trade; if automated, handle this specific prompt rather than piping unlimited
`yes` input. Require a swap signature and the intended on-chain result.
Swaps receiving SOL close the destination wrapped-SOL account, unwrapping any
existing balance there too; swaps spending SOL can leave a wrapped-SOL account.
