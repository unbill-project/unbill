---
core.desc: The Diesel SQLite implementation of LedgerStore with desktop directory ownership.
core.name: SQLite Store
core.category:
  - core.concept
core.belongs:
  - storage
  - workspace-layout
core.refines:
  - unbill-storage
---

`SqliteStore::open(root)` acquires an exclusive nonblocking `root/unbill.lock`
before opening SQLite on desktop, including Mac Catalyst. A competing Store
returns an I/O WouldBlock error. The lock handle lives with the shared database
so outstanding blocking operations retain ownership after the Store is dropped.
Failed startup releases the lock; normal exit and process crashes release it too.
The lock file is retained rather than unlinked. Android and non-Catalyst iOS
skip the file lock and use in-process device services.

Data lives in `root/unbill.sqlite3`. One Store owns multiple SQLite connections
for operations and revision observation. SQLite transactions still coordinate
those connections, and document saves merge the latest persisted snapshot.
Desktop CLI, TUI, Tauri, and Apple connect to the daemon through RPC.
The HTTP server owns its own locked data directory; browsers connect via HTTP.
Flat-file data is not automatically imported.

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

Dedicated identity and invitations tables expose only typed operations.
Identity initialization inserts only if absent. Device names persist only inside ledger snapshots.
Invitation creation generates a new token inside the backend, and consumption removes and returns one row atomically.
Existing migrations import supported legacy metadata and reject malformed or unknown rows without discarding them.

A new migration adds a singleton storage revision clock and explicit identity and invitations revisions,
plus a revision per ledger. Triggers update these within the mutation transaction, including deletions of metadata records.
A dedicated connection polls every 500 ms, reading revisions in one snapshot. The cursor advances only after
successful processing. Remote notifications may coalesce; duplicates are allowed. Local notifications follow commits.
The watcher stops when its store closes. Revision storage is bounded by ledger count and has no generic metadata fallback.

Tests cover desktop exclusion, competing process startup, crash release, failed startup,
outstanding-operation lock lifetime, stale-document merges, notifications, and durable reopening.
