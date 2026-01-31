# Security

This document describes the security model of `wmgr` and its integration with SVPI.

## Goals

- Reduce long-lived exposure of wallet secrets (mnemonics/private keys).
- Avoid storing secrets in `.wmgr`, environment variables, or other files.
- Keep common wallet operations practical for day-to-day usage.

## Threat model (assumptions)

`wmgr` is a local CLI. The model assumes:

- You trust the machine and OS user account running `wmgr`.
- You trust the `wmgr` binary you are running (use `wmgr self-hash` to verify).
- You trust the RPC endpoints you connect to, or you configure your own.

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

### In-memory only

`wmgr` does not write mnemonics/private keys to disk. Secrets are used in memory to derive a signer
and submit signed transactions.

### SVPI integration

When `--svpi` is used, `wmgr` executes SVPI in JSON mode (it expects the `svpi.response.v1`
envelope and reads `result.data` as the mnemonic/private key).

If `--svpi-pass` is not provided, `wmgr` prompts for the SVPI password with hidden input (no
terminal echo).

#### Important caveat: password is passed as a CLI argument

SVPI JSON mode requires `--password=...`. `wmgr` passes the password to `svpi` as a command-line
argument. Depending on OS and local permissions, other processes/users may be able to observe
arguments of running processes.

Operational recommendations:

- Avoid running `wmgr` on shared/multi-user hosts.
- Prefer locked-down environments (single user, minimal background tooling).
- Avoid redirecting stdout/stderr to log collectors when working with secrets.

#### Error paths and sensitive output

`wmgr` does not intentionally print secrets. However, if SVPI returns malformed JSON, the raw SVPI
output may be included in an error message. If that output contains sensitive data, it could be
displayed in the terminal.

## Configuration file (`.wmgr`)

The `.wmgr` file stores **defaults only** (mode, RPC endpoints, network/cluster, gas overrides,
slippage, and SVPI name/cmd/file). It does **not** store mnemonics/private keys or passwords.

Even without secrets, treat `.wmgr` as sensitive operational metadata (RPC URLs, named profiles).

## Network and RPC considerations

- Solana and EVM operations require RPC endpoints.
- Using a third-party RPC leaks metadata (IP address, timing, addresses queried). Use your own RPC
  when possible.
- Always verify destination addresses and amounts: `wmgr` is a signing client.

## Integrity verification (`self-hash`)

`wmgr self-hash` prints SHA256 hashes for:

- `wmgr` executable
- `config(.wmgr)` in the current directory
- (when SVPI mode is enabled) `svpi` executable and `config(.svpi)`

This is intended as a simple integrity check for binaries/configs in the current working directory.
