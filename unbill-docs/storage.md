---
name: Storage
desc: The review neighborhood for persisted ledger and device-local data.
category:
  - concept
belongs:
  - unbill
---

Storage entries describe the persistence boundary for whole Automerge ledger snapshots,
ledger metadata, and device-local metadata.

The store layer hides filesystem, memory, and remote-device details from higher layers.
Callers use typed store and metadata helpers instead of depending on paths,
device key names, or transport URLs directly.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from):
  - [fs-store](fs-store.md)
  - [ledger-doc](ledger-doc.md)
  - [memory-store](memory-store.md)
  - [unbill-storage](unbill-storage.md)

> **Sirno generated links end.**
