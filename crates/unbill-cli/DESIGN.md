# unbill-cli

A thin clap-driven command-line frontend for `UnbillService`. Useful for dogfooding, automated testing, and terminal users. Contains no business logic — all work is delegated to `unbill-core`.

## Commands

- `init` — initialize this device: generate a device key. Must be run once before any other command.
- `user create <display_name>` — add a fresh saved user (new user ID + display name) to this device. A device may hold many saved users.
- `user import <url>` — fetch an existing saved user from another device via an `unbill://user/...` URL and add it to this device's saved-user list. The other device must be online and have issued the URL via `user share`.
- `user list` — list all saved users stored on this device (user ID + display name for each).
- `user share --user-id <user_id>` — generate an `unbill://user/...` URL for a specific saved user so another device can import it via `user import`.
- `device show` — print this device's node ID and data directory.

- `ledger create | list | show | delete | invite | join` — ledger lifecycle. `ledger create` registers the creator's own device in `ledger.devices`. `ledger invite` generates an `unbill://join/...` URL authorizing a new device to access the ledger; `ledger join <url> [--label <name>]` dials the host, joins, and optionally stores a device-local label for the host device.
- `bill add | list | amend` — bill management. `bill amend` records a new version of an existing bill (same bill ID, all fields required); the latest version becomes the effective bill.
- `user add --ledger-id ...` and `user list --ledger-id ...` — managing named users in a ledger. Omitting `--ledger-id` operates on device-local saved users instead.
- `sync daemon | once | status` — P2P sync control. `sync once <peer_node_id>` dials a specific peer and syncs; `sync daemon` opens the endpoint and waits for incoming connections.
- `settlement <user_id>` — display who owes whom for a user, aggregated across all their ledgers.

Ledger and bill IDs are ULID strings on the command line. Most commands accept `--json` for machine-readable output, used in end-to-end tests.

## Invariants

- The binary never touches storage or network directly. All side effects go through `UnbillService`.
- Exit code 0 on success, non-zero on any error. Error messages go to stderr.

## Failure modes

- `UnbillError` variants are mapped to human-readable stderr messages.
- `sync once` exits non-zero if the peer is unreachable.
- `ledger join` exits non-zero if the host is offline or the token is invalid.
