---
core.desc: The multi-process Diesel SQLite implementation of LedgerStore.
core.name: SQLite Store
core.category:
  - core.concept
core.belongs:
  - storage
  - workspace-layout
core.refines:
  - unbill-storage
---

The `unbill-store-sqlite` crate provides `SqliteStore::open(root)` for concurrent processes on one machine.
Data lives in `root/unbill.sqlite3`. SQLite never creates or acquires `root/unbill.lock`;
concurrent access is coordinated entirely by SQLite database locks and transactions. All local application hosts use SQLite: the daemon, HTTP server, Apple bridge, and mobile Tauri.
CLI, TUI, desktop Tauri, and browser clients access their host through existing RPC or HTTP channels.
Each host keeps its existing data-directory choice. Flat-file data is not automatically imported.
Use matching application versions and stop writers before schema upgrades; network identity ownership and UI wiring are separate work.

Connections use WAL, synchronous FULL, and a five-second busy timeout. Migration discovery and execution
run in one immediate transaction. Initial WAL-mode contention is retried for up to five seconds.
Database operations execute on Tokio blocking workers.
Read-modify-write transactions acquire the writer lock before reading; timeout and rollback preserve committed data.
Cancellation can leave an already-started operation committed; its worker retains the database connection.

Document saves load and merge the latest Automerge snapshot in the transaction, validate ledger identity
and immutable fields, and atomically persist the merged document and its derived metadata.
Successful saves return merged state to the caller; failed saves preserve the caller document.
Standalone metadata saves retain document-derived fields and never reduce updated_at.
Metadata-only rows remain supported before document creation.

Dedicated identity, labels, and invitations tables expose only typed operations.
Identity initialization inserts only if absent; competing changes to a label follow commit order.
Invitation creation generates a new token inside the backend, and consumption removes and returns one row atomically.
Existing migrations import supported legacy metadata and reject malformed or unknown rows without discarding them.

A new migration adds a singleton storage revision clock and explicit identity, labels, and invitations revisions,
plus a revision per ledger. Triggers update these within the mutation transaction, including deletions of metadata records.
A dedicated connection polls every 500 ms, reading revisions in one snapshot. The cursor advances only after
successful processing. Remote notifications may coalesce; duplicates are allowed. Local notifications follow commits.
The watcher stops when its store closes. Revision storage is bounded by ledger count and has no generic metadata fallback.

Tests use synchronized child processes for concurrent startup, merges, metadata, identity, label conflicts,
invitation consumption, notifications, lock timeouts, writer crashes, and reopening durable data.
