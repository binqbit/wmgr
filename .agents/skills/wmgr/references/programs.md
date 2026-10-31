# Solana programs and signing roles

`wmgr program` manages Solana upgradeable loader-v3 programs. It does not deploy
EVM contracts or execute arbitrary application initialization/admin instructions.
Distinguish a program's upgrade authority from its application owner/admin or a
multisig; those roles may require separate project tools and signatures.

## Role mapping

| Command | Program SVPI record | Required private signers |
| --- | --- | --- |
| `deploy` | Program keypair/private material | Program, payer, upgrade authority |
| `upgrade` | Keypair or public-only record | Payer, current upgrade authority |
| `show` | Keypair or public-only record | None, but reading the record still needs its SVPI password |
| `set-authority` | Keypair or public-only record | Payer, current authority, **new authority** |
| `close-buffer` | Not used | Payer, buffer authority |

`--program`, `--payer`, `--authority`, and `--new-authority` take SVPI **record
names**, not literal addresses. `--buffer` takes a literal public address. Assign
roles from the project requirements and current on-chain state. Do not collapse
separate authorities just to reduce password prompts.

Missing file and role names prompt. Payer defaults in the prompt to the configured
SVPI wallet and authority to payer, but they still prompt; provide every name in
scripts. Each distinct record is loaded once per command. Without `--svpi-pass`,
it gets a separate hidden password prompt even when several records share a password.
One `--svpi-pass` value applies to **all** records loaded in that invocation; it
cannot supply a role-to-password map. Different-password records require the
native individual prompts or a separately designed integration. There is no
cross-command cache or unlocked session.

Bare mnemonic records always trigger an additional BIP39 passphrase prompt.
Structured/recovery mnemonic records first try an empty passphrase and prompt
after a mismatch. Program commands have no `--seed-passphrase` option. Therefore
`--svpi-pass` alone cannot guarantee a completely unattended program operation.

## Commands

Examples omit passwords and use native hidden prompts. Add the shared password
wrapper only when its assumptions hold; see [automation.md](automation.md).

```sh
wmgr program deploy "$program_so" \
  --program "$program_record" --payer "$payer_record" --authority "$authority_record" \
  --cluster "$cluster" --rpc "$rpc"

wmgr program upgrade "$program_so" \
  --program "$program_record" --payer "$payer_record" --authority "$authority_record" \
  --cluster "$cluster" --rpc "$rpc"

wmgr program show --program "$program_record" --cluster "$cluster" --rpc "$rpc"

wmgr program set-authority \
  --program "$program_record" --payer "$payer_record" --authority "$authority_record" \
  --new-authority "$next_authority_record" --cluster "$cluster" --rpc "$rpc"

wmgr program close-buffer --buffer "$buffer_address" \
  --payer "$payer_record" --authority "$buffer_authority_record" \
  --cluster "$cluster" --rpc "$rpc"
```

All support SVPI file/command/password options and Solana commitment. Deploy and
upgrade accept `--timeout SECONDS` (positive, default 1200),
`--compute-unit-price PRICE` (micro-lamports per compute unit), and
`--buffer ADDRESS` for resuming an upload. The input must be a compiled regular
ELF `.so`. Build/inspect it with the project's existing toolchain first.

Deploy requires an unused program address. Upgrade requires an existing program
and matching current authority, and cannot upgrade an immutable program. Do not
silently convert one operation into the other. `set-authority` requires the new
authority's signature as well as the current one's; a public-only new-authority
record is insufficient. Finalizing/removing authority is not an exposed command.

## Sequence and recovery

For several programs, map every artifact, expected program address, SVPI program
record, fee payer, authority, and dependency separately. Pin network and RPC, check
fee funding, and verify each program before dependent initialization or authority
transfer. Do not change authorities as an unrequested cleanup step.

The buffer address is printed before upload. Preserve it with the step record.
After a timeout/error, query the program and transaction state before retrying:
an operation may have completed despite missing local confirmation. `program show`
reports address, authority, slot, data length, and balance; those fields alone do
not prove the deployed bytes equal the requested artifact. Use the project's
appropriate on-chain code verification when that is the required postcondition.

Resume with `--buffer` only when the buffer exists, has the expected authority,
and is large enough. If an upgrade requires extending program storage, the current
implementation requires a fresh buffer instead. Preserve the old buffer address
and reclaim it separately when authorized. `close-buffer` sends its SOL to the
selected payer; it is a transaction, not a read-only recovery check, and refuses
program accounts. Do not automatically close unrelated or possibly active buffers.
