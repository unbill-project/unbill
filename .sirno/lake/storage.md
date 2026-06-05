---
core.name: Storage
core.desc: The review neighborhood for persisted ledger and device-local data.
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
