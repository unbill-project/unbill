# unbill-cli

A thin clap-driven command-line frontend for `UnbillService`. Useful for dogfooding, automated testing, and terminal users. Contains no business logic — all work is delegated to `unbill-core`.

## Commands

- `init` — initialize the device: generate a key, set display name.
- `ledger create | list | show | delete` — ledger lifecycle. `ledger create` also registers the creator's own device in `ledger.devices` so it can sync with peers.
- `bill add | list | amend | delete | restore` — bill management.
- `member list | add | remove | invite | join` — group membership. `invite` generates an invite URL; `join` accepts one.
- `sync daemon | once | status` — P2P sync control. `sync once <peer_node_id>` dials a specific peer and syncs; `sync daemon` opens the endpoint and waits for incoming connections.
- `settlement <user_id>` — display who owes whom for a user, aggregated across all their ledgers.

Ledger and bill IDs are ULID strings on the command line. Most commands accept `--json` for machine-readable output, used in end-to-end tests.

## Invariants

- The binary never touches storage or network directly. All side effects go through `UnbillService`.
- Exit code 0 on success, non-zero on any error. Error messages go to stderr.

## Failure modes

- `UnbillError` variants are mapped to human-readable stderr messages.
- `sync once` exits non-zero if the peer is unreachable.
- `member join` exits non-zero if the host is offline or the token is invalid.
