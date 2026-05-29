---
name: Memory Store
desc: The in-process LedgerStore used by tests.
category:
  - concept
belongs:
  - storage
  - workspace-layout
refines:
  - unbill-storage
---

`unbill-store-memory` provides `InMemoryStore` for tests.
It is not intended for production use.

The store holds ledger data and device metadata in hash maps behind a mutex.
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

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [storage](storage.md)
  - [workspace-layout](workspace-layout.md)
- belongs (from): (none)

> **Sirno generated links end.**
