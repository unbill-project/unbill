---
core.name: Unbill CLI
core.desc: The thin command-line frontend for scripting, dogfooding, and e2e verification.
core.category:
  - core.concept
core.belongs:
  - applications
core.refines:
  - device-console-split
---

`unbill-cli` is a terminal frontend for scripting,
dogfooding,
and end-to-end verification.
It is a thin client.
It connects to a running `unbill-daemon` over the local socket and forwards commands over RPC.

The surface covers device initialization and display,
ledger create, list, show, invite (with terminal QR code), join, and devices,
bill add, list, amend, and conflicts,
user create, add, list, archive, and restore,
one-shot peer sync (sync status is stubbed for a future milestone),
and settlement for one user across every ledger where that user appears.

The CLI owns parsing, formatting, and exit codes only.
Storage, validation, sync, and settlement stay in `unbill-console`,
reached through the daemon.
IDs and node identities remain opaque strings until parsed by the CLI or core types.
`--json` is the stable machine-readable surface for scripts and end-to-end tests.

Invalid IDs,
invalid amounts,
and invalid node IDs fail before service calls.
Amount parsing rejects cent-conversion overflow and invalid UTF-8 fraction boundaries.
Strict Clippy exceptions stay local to character-boundary truncation and bounded display numbering.
Service errors surface as non-zero exits with human-readable stderr.
Invitation QR encoding errors also propagate to stderr and a non-zero exit; the URL is printed first.
Commands fail clearly when the daemon socket cannot be reached.

Implementation:
`main.rs` parses with Clap,
connects through `RpcAsymChannel` at the socket under `UNBILL_PATH`,
creates `UnbillConsole`,
and dispatches to `commands.rs`.
`commands.rs` maps arguments to typed service inputs.
`output.rs` owns human-readable and JSON output shapes.

End-to-end tests run the real binary against isolated temporary data directories.
Each test environment starts `unbill-daemon`,
waits for the daemon readiness line with the node ID,
uses the temp socket,
and kills the daemon on drop.
The suite covers JSON output,
multi-process persistence,
join,
and one-shot peer sync flows.

Ledger show includes the console-generated emoji fingerprint beneath the ledger
name and exposes it as emoji_fingerprint in JSON output.

Joining requires `--label`, the joining device's shared name in that ledger.
The ledger devices command includes each device's shared label in text and JSON.

Ledger-scoped user archive and restore commands change shared user metadata.
User JSON includes archived, and text listings mark archived memberships.
