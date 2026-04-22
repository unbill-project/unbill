# Unbill CLI

Terminal frontend for `UnbillService`. It exists for dogfooding, scripting, and end-to-end verification.

## Surface

- `init` prints or creates the local device identity
- `device show` reports the device ID and data directory
- `ledger create | list | show | delete | invite | join` covers the ledger lifecycle and device join flow
- `bill add | list | amend` manages effective bills in one ledger
- `user create | import | list | share | delete` manages saved users on this device
- `user add --ledger-id ...` and `user list --ledger-id ...` manage users inside one ledger
- `sync daemon | once | status` exposes peer-to-peer sync control
- `settlement <user_id>` prints the net settlement for one user across every ledger they appear in

The CLI has two distinct user concepts and the docs should say so plainly:

- saved users are device-local identities used for convenience and transfer between devices
- ledger users are shared records inside one ledger and are the identities referenced by bills

## Rules

- the CLI owns parsing, formatting, and exit codes only
- storage, validation, sync, and settlement stay in `unbill-core`
- IDs and node identities are treated as opaque input strings until parsed by the CLI or core
- `--json` is the stable machine-readable surface for scripts and end-to-end tests

## Failure model

- invalid IDs, invalid amounts, and invalid node IDs fail before calling the service
- service errors surface as non-zero exits with human-readable stderr
- join and user-import commands fail if the remote device is offline or the provided URL is invalid
