# unbill-cli

A thin clap-driven command-line frontend for `UnbillService`. Useful for dogfooding, automated testing, and terminal users. Contains no business logic — all work is delegated to `unbill-core`.

## Commands

- `init` — initialize the device: generate a key, set display name.
- `ledger create | list | show | export | import | delete` — ledger lifecycle.
- `bill add | list | amend | delete | restore` — bill management.
- `member list | add | remove` — group membership. `invite` and `join` are available from M4.
- `sync daemon | once | status` — P2P sync control.
- `settlement <user_id>` — display who owes whom for a user, aggregated across all their ledgers.

Ledger and bill IDs are ULID strings on the command line. Most commands accept `--json` for machine-readable output, used in end-to-end tests.

## Invariants

- The binary never touches storage or network directly. All side effects go through `UnbillService`.
- Exit code 0 on success, non-zero on any error. Error messages go to stderr.

## Failure modes

- `UnbillError` variants are mapped to human-readable stderr messages.
- Network timeouts in `sync once` print a warning and exit non-zero.

## Open questions

- `sync daemon`: run in the foreground (current plan) or support backgrounding via fork?
