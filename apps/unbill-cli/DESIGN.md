# Unbill CLI

Terminal frontend for `UnbillConsole`. It exists for dogfooding, scripting, and end-to-end verification.

## Surface

- `init` prints or creates the local device identity
- `device show` reports the device ID and data directory
- `ledger create | list | show | delete | invite | join | devices` covers the ledger lifecycle, device join flow, and device membership
- `bill add | list | amend` manages effective bills in one ledger
- `user create --ledger-id ...` creates a new user and adds them to a ledger
- `user add --ledger-id ...` adds an existing user (by ID) to a ledger
- `user list` lists all unique users across every ledger on this device; `user list --ledger-id ...` lists users in one ledger
- `sync daemon | once | status` exposes peer-to-peer sync control
- `settlement <user_id>` prints the net settlement for one user across every ledger they appear in

## Rules

- the CLI owns parsing, formatting, and exit codes only
- storage, validation, sync, and settlement stay in `unbill-console`
- IDs and node identities are treated as opaque input strings until parsed by the CLI or core
- `--json` is the stable machine-readable surface for scripts and end-to-end tests

## Failure model

- invalid IDs, invalid amounts, and invalid node IDs fail before calling the service
- service errors surface as non-zero exits with human-readable stderr
- join commands fail if the remote device is offline or the provided URL is invalid
