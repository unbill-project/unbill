---
core.name: Memory Store
core.desc: The in-process LedgerStore used by tests.
core.category:
  - core.concept
core.belongs:
  - storage
  - workspace-layout
core.refines:
  - unbill-storage
---

`unbill-store-memory` provides `InMemoryStore` for tests.
It is not intended for production use.

The store holds ledger data, typed label and invitation maps, and an optional secret key behind a mutex.
All metadata operations use the same typed interface as the filesystem and SQLite backends.
Invitation consumption removes and returns one entry atomically.
All operations are synchronous under that lock and do no I/O.

Saved ledgers pair `LedgerMeta` with serialized document bytes.
The store serializes through `LedgerDoc::save` and deserializes on load,
so tests exercise the same round-trip path as real stores.

Every successful `save_ledger` emits `ServiceEvent::LedgerUpdated`,
matching the contract of production stores.
`subscribe()` returns a real broadcast receiver.

`create_secret_key` is idempotent.
`get_secret_key` is supported,
unlike remote stores that cannot expose raw key material.

The crate is exercised by higher-level tests that use it as a dependency.

Invitation creation generates a fresh token inside the backend; callers cannot resubmit consumed tokens.
Typed metadata mutations emit process-local invalidation events after persistence.
