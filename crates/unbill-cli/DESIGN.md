# unbill-cli

Terminal frontend for `UnbillService`. It exists for dogfooding, scripting, and end-to-end verification.

## Command groups

- `device` — initialize and inspect the local device
- `ledger` — create, list, inspect, delete, invite, and join ledgers
- `bill` — add, list, and amend bills
- `user` — manage saved users on the device and users inside a ledger
- `sync` — run the daemon, sync once with a peer, inspect status
- `settlement` — print a user's net settlement across ledgers

## Rules

- the CLI owns parsing, formatting, and exit codes only
- storage, validation, sync, and settlement stay in `unbill-core`
- `--json` is the stable machine-readable output for scripts and tests
