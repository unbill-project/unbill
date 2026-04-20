# unbill — Implementation

## Workspace

- `crates/unbill-core/` — domain model, Automerge ledger, storage, sync, settlement, service facade
- `crates/unbill-cli/` — terminal frontend
- `crates/unbill-tauri/` — Tauri IPC bridge
- `apps/unbill-ui-leptos/` — shared Leptos UI
- `apps/unbill-desktop/` — early React desktop shell

## Core shape

- `model/` — typed IDs and domain structs
- `doc/` — `LedgerDoc`, the in-memory Automerge wrapper
- `storage/` — `LedgerStore`, `FsStore`, and `InMemoryStore`
- `net/` — sync, join, and saved-user transfer over Iroh
- `service/` — `UnbillService`, the main orchestration API
- `settlement/` — balance accumulation and minimum-cash-flow reduction

## Runtime rules

- Ledgers persist as full Automerge snapshots plus small metadata files.
- Device-local metadata stores keys, labels, saved users, and pending tokens.
- Sync is session-based: peers negotiate shared ledgers, run Automerge sync, save touched docs, and disconnect.
- Bills use integer cents and weighted shares; settlement runs on effective bills only.
- Frontends consume typed service or IPC APIs and do not own business rules.
