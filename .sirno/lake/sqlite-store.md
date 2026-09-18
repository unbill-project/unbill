---
core.desc: The single-owner Diesel SQLite implementation of LedgerStore.
core.name: SQLite Store
core.category:
  - core.concept
core.belongs:
  - storage
  - workspace-layout
core.refines:
  - unbill-storage
---

The `unbill-store-sqlite` crate provides `SqliteStore::open(root)`.
It stores data in `root/unbill.sqlite3` and holds `root/unbill.lock` exclusively for its lifetime on every platform.
The lock is shared with FsStore on desktop, so the two backends cannot accidentally operate in the same directory concurrently.
Existing applications continue using FsStore; selecting SQLite and importing flat-file data are separate work.

Embedded Diesel migrations create two tables: `ledgers(id TEXT PRIMARY KEY NOT NULL, metadata BLOB, document BLOB)` and `device_metadata(key TEXT PRIMARY KEY NOT NULL, value BLOB NOT NULL)`.
Metadata uses the filesystem backend JSON representation; documents remain whole Automerge snapshots.
Nullable ledger columns allow independent metadata and document saves; each upsert preserves the other column.
Device identity creation inserts only if absent.

A single Diesel connection is serialized behind a mutex and all database operations run on Tokio blocking workers.
Blocking work retains ownership of the directory lock even if its async caller is cancelled.
Successful document saves publish process-local LedgerUpdated events after the database write.
There is no cross-process polling or concurrent backend support.
Tests cover persistence, independent column updates, device identity, events, invalid data, and exclusive ownership.
