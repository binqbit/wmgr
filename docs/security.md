# Security

This document describes the security model of `wmgr` and its integration with SVPI.

## Goals

- Reduce long-lived exposure of wallet secrets (mnemonics/private keys).
- Keep secrets out of `.wmgr`, command output, and environment variables; write
  local wallet backups when SVPI is skipped, fails, or an export is requested.
- Keep common wallet operations practical for day-to-day usage.

## Threat model (assumptions)

`wmgr` is a local CLI. The model assumes:

- You trust the machine and OS user account running `wmgr`.
- You trust the `wmgr` binary you are running (use `wmgr self-hash` to verify).
- You trust the RPC endpoints you connect to, or you configure your own.
- For GPU vanity search, you trust the installed OpenCL runtime and device driver.

Out of scope (not solvable by `wmgr`):

- OS/user compromise (keyloggers, malware, memory scraping).
- Supply-chain compromise of your toolchain or dependencies.

## Secret handling

### Key sources

`wmgr` can resolve keys from:

- **Solana:** `--keyfile`, `--seed`, or `--svpi`
- **EVM:** `--privkey`, `--privkey-file`, `--seed`, or `--svpi`

Security note: passing secrets via CLI flags (e.g. `--seed`, `--privkey`, `--svpi-pass`) may leak
into shell history, terminal scrollback, logs, or process listings. Prefer `--svpi` + interactive
password prompt.

### Offline wallet generation

Generation uses OS cryptographic randomness and stops if it is unavailable. GPU search
receives private seeds; `--backend cpu` keeps them off the device. Secret memory is cleared
where practical, without a guarantee after driver failure or abrupt termination.

Local backups are plaintext, never overwrite existing files, and use `0600` on Unix
(default ACLs elsewhere). Keep them outside repositories and shared directories.
BIP39 passphrases are never saved and do not encrypt backups; a JSON keypair can sign
without the passphrase. Secrets are not printed or stored in `.wmgr`.

### SVPI integration

SVPI saves use password encryption and are verified by reading the record back. Existing
names are refused, but SVPI cannot prevent concurrent writers from claiming the same name.

Passwords are prompted without echo and preserved exactly. SVPI receives passwords and
payloads through process arguments, which other local processes may inspect. Use a trusted
machine. Raw SVPI output is not echoed in errors.

## Configuration file (`.wmgr`)

The `.wmgr` file stores **defaults only** (mode, RPC endpoints, network/cluster, gas overrides,
slippage, and SVPI name/cmd/file). It does **not** store mnemonics/private keys or passwords.

Even without secrets, treat `.wmgr` as sensitive operational metadata (RPC URLs, named profiles).

## Network and RPC considerations

- Balance queries and transactions require RPC endpoints. Key generation is local;
  SVPI accesses its configured file or device.
- Using a third-party RPC leaks metadata (IP address, timing, addresses queried). Use your own RPC
  when possible.
- Always verify destination addresses and amounts: `wmgr` is a signing client.

## Integrity verification (`self-hash`)

`wmgr self-hash` prints SHA256 hashes for:

- `wmgr` executable
- `config(.wmgr)` in the current directory
- (when SVPI mode is enabled) `svpi` executable and `config(.svpi)`

This is intended as a simple integrity check for binaries/configs in the current working directory.
