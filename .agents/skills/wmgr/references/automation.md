# Shell and Python coordination

Read [commands.md](commands.md) for each operation being composed. Choose the
smallest script appropriate to the task; a simple command does not need a workflow
engine, a manifest format, or new dependencies.

## Prepare a concrete sequence

For distributions and other multi-wallet flows, represent each step with an ID,
chain, asset, source record and known public address, recipient, decimal amount,
dependencies, and required evidence. Public inputs can be script parameters or
a task-specific plan file. Do not persist wallet secrets alongside them.

Before executing, validate every row, not just the first: network/endpoint
identity, recipient syntax, asset identity and decimals, source identity, total
spend, fee reserve, and funding dependencies. Reject duplicate rows unless the
user intentionally requested repeated transfers. Determine rounding/remainder
allocation for percentage distributions. Reuse known mappings; ask about missing
business choices instead of choosing recipients, amounts, or signing roles.

Provide a plan-only path for a multi-step money-moving script. This is wrapper
behavior, not a `wmgr --dry-run` flag. It must never call send, trade, deployment,
authority-change, or wallet-write operations. It should not request a password.
Make actual execution a distinct, intentional invocation when the user's task is
to prepare a script. Existing explicit execution authorization still applies.

## Password reuse

For one ordinary operation, omit `--svpi-pass` and use the native hidden prompt.
For several operations with the same SVPI password, prompt once in the wrapper,
keep it in process memory, and pass it to each child using `--svpi-pass=VALUE`.
The equals form also preserves passwords beginning with a hyphen.

Do not trim or normalize the password, interpolate it into shell source, print
the child argument list, export it into the environment, or write it to a file.
Disable shell tracing before reading it. A Python `CalledProcessError` contains
the full argument list; replace its default rendering with a redacted error.
Do not log tracebacks/local variables that may contain secrets.

This prevents accidental source/history/log exposure, but **does not hide process
arguments**. Both `wmgr --svpi-pass` and its SVPI subprocess expose password
arguments to observers with sufficient local access. Even the native hidden
prompt ends in an SVPI password argument. The CLI has no password-fd/stdin/file,
environment-password, agent, or session-unlock interface. A requirement to avoid
argument exposure needs a separately agreed implementation change, not a fictitious
flag. Memory cleanup in shell/Python is best-effort, not guaranteed zeroization.

One password cannot decrypt differently encrypted records. If records use
different passwords, collect the necessary passwords separately and map them
to records; do not change vault encryption to simplify a script. For program
commands, see the additional shared-password restriction in [programs.md](programs.md).
BIP39 passphrases are a separate input with command-specific handling.

## Invocation building blocks

These are argument/password handling building blocks for an already validated
plan, not complete distribution scripts. Add task-specific preflight, execution
selection, progress recording, and reconciliation around them. Never call them
merely to validate a money-moving command's syntax.

### POSIX sh

Keep expansions quoted and use positional arguments (`"$@"`), not command
strings or `eval`. POSIX `read` has no `-s`; use a controlling terminal and restore
its state on errors/signals. This wrapper accepts only subcommands supporting
`--svpi-pass`; use the ordinary CLI for public balance queries, config, and help.

```sh
#!/bin/sh
set +x
set +a
set -eu

wmgr_bin=${WMGR_BIN:-wmgr}
# Remove any export attribute inherited from the launching environment.
unset wmgr_password
wmgr_password=
wmgr_tty_state=
cleanup() {
    if [ -n "$wmgr_tty_state" ]; then
        stty "$wmgr_tty_state" < /dev/tty || :
    fi
    unset wmgr_password
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

# Call only on the execution path, after validating the public plan.
read_wmgr_password() {
    wmgr_tty_state=$(stty -g < /dev/tty) || return 1
    stty -echo < /dev/tty || return 1
    printf 'SVPI password: ' > /dev/tty
    if IFS= read -r wmgr_password < /dev/tty; then
        wmgr_read_status=0
    else
        wmgr_read_status=1
    fi
    stty "$wmgr_tty_state" < /dev/tty || return 1
    wmgr_tty_state=
    printf '\n' > /dev/tty
    [ "$wmgr_read_status" -eq 0 ] && [ -n "$wmgr_password" ]
}

run_wmgr_authenticated() {
    if "$wmgr_bin" "$@" "--svpi-pass=$wmgr_password"; then
        return 0
    else
        wmgr_status=$?
        printf 'wmgr failed (exit %s); reconcile before retrying.\n' "$wmgr_status" >&2
        return "$wmgr_status"
    fi
}
```

After validation and password input, an invocation is
`run_wmgr_authenticated send "$amount" sol "$recipient" --svpi --svpi-name "$source_record" --cluster "$cluster" --rpc "$rpc"`.
When placing calls inside conditionals, check return codes explicitly; shell
`set -e` is not a substitute for correct failure handling. Supply `--svpi-file`
and `--svpi-cmd` when the plan pins them.

### Python

Use an argument list and `shell=False`. Require a terminal for hidden input;
`getpass` can otherwise fall back to echoed input. Do not print a raw exception
whose command contains the password.

```python
import getpass
import subprocess
import warnings


def read_wmgr_password():
    with warnings.catch_warnings():
        warnings.simplefilter("error", getpass.GetPassWarning)
        try:
            password = getpass.getpass("SVPI password: ")
        except (getpass.GetPassWarning, EOFError):
            raise RuntimeError("A terminal is required for hidden password input") from None
    if not password:
        raise ValueError("SVPI password must not be empty")
    return password


def run_wmgr_authenticated(executable, args, password, *, cwd):
    try:
        result = subprocess.run(
            [executable, *args, "--svpi-pass=" + password],
            cwd=cwd,
            shell=False,
            check=False,
        )
    except OSError:
        raise RuntimeError("Could not start wmgr") from None
    if result.returncode != 0:
        raise RuntimeError(
            f"wmgr failed (exit {result.returncode}); reconcile before retrying"
        )
    return result.returncode
```

Collect the password once on the execution path and pass it to consecutive calls.
Use `try/finally` to drop wrapper references when done; this does not erase all
copies in memory. For different passwords, select by source record rather than
silently reusing the first one. Keep RPC URLs containing credentials out of shared
logs: `wmgr` can print its endpoint even if the wrapper hides its argument list.

## Progress, failure, and verification

For a resumable multi-step script, record public step IDs and outcomes durably:
pending, attempting, confirmed, failed-before-submission when established, or
uncertain. Include returned transaction identifiers and confirmed chain evidence.
Write the attempting state before invoking the child so an interrupted process
cannot masquerade as an untouched row. Do not record secrets or raw child argv.
Bind saved progress to the exact plan, chain, asset, and source identity. Prevent
concurrent runs against that state and use atomic updates so a crash cannot
silently reset a completed or uncertain step to pending.

Stop on a failed dependency. Already completed transactions remain completed;
the sequence is not atomic. A nonzero status, process timeout, or missing hash
does not prove that nothing was submitted. Refuse automatic replay of attempting
or uncertain steps until chain reconciliation establishes the outcome. A balance
change alone may not identify a specific transfer when other activity exists.

Use signatures/receipts and operation-specific postconditions. EVM needs an
independent receipt/status check; swaps can be aborted with exit zero; wallet
saves can succeed only as a local fallback. Do not label an entire sequence
successful because every child process returned zero.

Test shell syntax with the matching interpreter and mock the executable for
ordering, quoting, passwords with spaces/special characters, first/middle-step
failures, and restart after partial completion. Mock RPC/SVPI when testing actual
CLI behavior. Keep synthetic tests distinct from real vault access or chain
transactions. Explain to the user how to inspect the plan, execute it, and
reconcile a partial run.
