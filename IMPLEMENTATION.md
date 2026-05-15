# Unbill — Implementation

## Workspace

### Core crates

- `crates/unbill-model/` — domain types and typed IDs; no logic, no I/O
- `crates/unbill-storage/` — `LedgerStore` trait and `LedgerDoc` Automerge wrapper
- `crates/unbill-store-fs/` — filesystem-backed `LedgerStore`
- `crates/unbill-store-memory/` — in-memory `LedgerStore` for tests
- `crates/unbill-store-http/` — HTTP-backed `LedgerStore` for the browser web frontend
- `crates/unbill-event/` — `ServiceEvent` broadcast types

### Channel crates

- `crates/unbill-symmetric-channel/` — Iroh endpoint, device-to-device sync and join protocols
- `crates/unbill-asymmetric-channel/` — `AsymChannel` trait (device ↔ console) and implementations: in-process (`local`), tarpc RPC (`rpc`), HTTP client (`http`)

### Role crates

- `crates/unbill-service/` — device-side service; owns the store, sym channel endpoint, and implements the asym channel server side
- `crates/unbill-console/` — console-side library: CRDT document operations, settlement, conflict detection

### UI crates

- `crates/unbill-tauri/` — Tauri IPC bridge; hosts `unbill-ui-native` as the default desktop shell
- `crates/unbill-ui-components/` — shared Leptos UI components used by both frontend apps

### Applications

- `apps/unbill-cli/` — command-line frontend
- `apps/unbill-tui/` — keyboard-driven TUI frontend (Ratatui)
- `apps/unbill-server/` — HTTP server implementing the REST API consumed by `unbill-store-http`
- `apps/unbill-ui-native/` — default desktop UI built with Leptos, hosted by Tauri
- `apps/unbill-ui-remote/` — browser web frontend built with Leptos

## Runtime rules

- Ledgers persist as full Automerge snapshots plus small metadata files.
- Device-local metadata stores keys, labels, saved users, and pending tokens.
- Sym sync is session-based: peers negotiate shared ledgers, run Automerge sync, save touched docs, and disconnect.
- Asym sync runs one Automerge sync round per console request over the `AsymChannel`.
- Bills use integer cents and weighted shares; settlement runs on effective bills only.
- Consoles consume the `AsymChannel` API and do not own business rules or persistence.
