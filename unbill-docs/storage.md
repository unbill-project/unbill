---
core.desc: The review neighborhood for persisted ledger and device-local data.
core.name: Storage
core.category:
  - core.concept
core.belongs:
  - unbill
---

Storage entries describe the persistence boundary for whole Automerge ledger snapshots,
ledger metadata, and device-local metadata.

The store layer hides filesystem, memory, and remote-device details from higher layers.
Callers use typed store and metadata helpers instead of depending on paths,
device key names, or transport URLs directly.

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [unbill](unbill.md)
- core.belongs (from):
  - [fs-store](fs-store.md)
  - [ledger-doc](ledger-doc.md)
  - [memory-store](memory-store.md)
  - [unbill-storage](unbill-storage.md)

> **Sirno generated links end.**
