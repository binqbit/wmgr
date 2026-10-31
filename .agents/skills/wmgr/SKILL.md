---
name: wmgr
description: "Explain and use the wmgr Wallet Manager CLI for Solana and EVM wallets, balances, transfers, distributions, Raydium trades, and Solana program management. Use when composing wmgr commands or coordinating them in shell or Python scripts, including SVPI passwords and signer roles."
---

# wmgr

Help the user perform the requested wallet operation with the actual `wmgr` CLI.
Support explanations, individual commands, and coordinated shell or Python scripts.
Deployment is one use case; ordinary wallet and transfer workflows are equally in scope.
Respond in the user's language.

## Choose the relevant guidance

- Read [commands.md](references/commands.md) for command syntax, key sources,
  wallet storage, balances, transfers, and trading behavior.
- Read [automation.md](references/automation.md) when writing scripts, distributing
  funds, reusing passwords, coordinating multiple wallets, or handling partial failure.
- Read [programs.md](references/programs.md) for Solana deployment, upgrades,
  upgrade authorities, and upload buffers.

These references describe the repository's v3.4 CLI. Before relying on a flag,
check the selected executable with `wmgr --version`, `wmgr --help`, and the relevant
subcommand's `--help`. When working in the wmgr repository, use `README.md`,
`docs/`, and the matching code under `src/app/` and `src/infra/` to resolve gaps.
An installed binary may differ from the checkout. A help/version failure is a
failed executable check, not permission to try a money-moving command.

## Understand the requested operation

Use existing project decisions and known public addresses first. Establish only
the missing details that affect the result:

- Explanation, script preparation, or execution; authorization to prepare a
  script alone does not authorize sending transactions.
- Chain and exact RPC endpoint, asset or token contract, amounts, recipients,
  source wallets, and required signing roles.
- For a sequence: dependencies, fee funding, success evidence, and what to do if
  a step fails after earlier steps have completed.

For a simple question, give the command and explain its relevant options.
For multiple operations, make the mapping explicit, for example:

| Step | Network / asset | Source SVPI record | Recipient / target | Amount | Depends on |
| --- | --- | --- | --- | --- | --- |
| Fund worker | Chosen Solana cluster / SOL | treasury | Worker's public address | User's amount | Verified fee reserve |
| Distribute | Same cluster / USDC | worker | Recipient's public address | User's amount | Funding confirmed |

Record names select secrets in SVPI. Public addresses identify recipients and
on-chain accounts. They are not interchangeable. A shared password does not
make different wallet records the same signer. Resolve the public identity and
its project role before authorizing a transaction.

## Respect the CLI's capabilities

`wmgr` signs internally for its supported sends, swaps, and Solana program
operations. It has no general `sign` command for arbitrary transactions,
messages, typed data, multisig proposals, or application contract calls. It also
has no native batch-transfer command, generic SPL-token transfer, ERC20 balance
command, or EVM contract deployment command. Explain a missing capability and
use the project's existing SDK/tool when appropriate; do not invent flags or
export private keys just to make an unsupported operation look supported.

Use one CLI process per scripted operation. The REPL is for interactive use;
it catches individual errors and does not provide reliable batch exit status.
There is no general JSON output, `--dry-run`, `--yes`, or unlock-session interface.
A script's own plan-only mode must skip signing/broadcast calls entirely.

## Bind the network and signer explicitly

- Solana defaults to `mainnet-beta`; EVM sends default to the configured network
  or Ethereum mainnet. `balance` enters EVM mode only with `--network`.
- Pin both `--cluster` and `--rpc` for Solana, or `--network` and `--rpc` for EVM.
  These options resolve independently; setting a cluster alone can retain a
  saved RPC for a different network. Verify the endpoint's actual chain identity
  through an appropriate read-only RPC query when executing transactions.
- `.wmgr` and SVPI configuration resolve from the working directory. Choose that
  directory intentionally. `.wmgr` is binary configuration; manage it with
  `wmgr config`, not by editing it or storing passwords in it.
- General signing commands use `--svpi --svpi-name NAME`. Program commands use
  named roles such as `--program`, `--payer`, and `--authority` and always use SVPI.
- Prefer explicit public-address balance queries when no signer is needed.
  They do not need a vault unlock.

## Handle passwords without changing their meaning

Let a person type secrets into a hidden local prompt, never into chat, generated
source, shell history, or reports. For one command, the native hidden prompt is
usually sufficient. For a coordinated script, follow the password patterns in
[automation.md](references/automation.md).

`--svpi-pass` supplies a password; it does not authorize unrelated operations or
unlock later processes. One shared password works only for records actually
encrypted with that password. Keep BIP39 passphrases separate from SVPI passwords.

The current integration passes passwords to SVPI in process arguments, including
after a native hidden prompt. A wrapper cannot promise argument-free secret
handling. Explain this concrete limitation when designing password automation;
do not silently replace it with plaintext key exports or new credential storage.

## Verify the requested result

Validate generated scripts with synthetic inputs and mocked commands before live
use. Check argument boundaries, decimal precision, operation order, failure
propagation, secret redaction, and partial-completion behavior. For POSIX `sh`,
avoid Bash-only syntax; if Bash is needed, use a Bash shebang and say so.

Match success evidence to the operation. Exit status alone is insufficient for
aborted swaps, wallet saves that fall back to local files, and current EVM sends.
See the reference for each command. A timeout after submission has an unknown
outcome until reconciled; never automatically replay the whole sequence.

Report what was prepared, locally checked, actually executed, and verified on
chain. Preserve previously granted authorization without asking again for the
same action, and do not expand it to other recipients, networks, amounts, or roles.
