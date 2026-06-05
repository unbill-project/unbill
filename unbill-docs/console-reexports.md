---
core.desc: The compatibility modules that expose storage and network types through unbill-console.
core.name: Console Reexports
core.category:
  - core.concept
core.belongs:
  - unbill-console
---

`unbill-console` keeps thin compatibility modules for storage and network-facing names.

The storage module re-exports the `LedgerStore` boundary and storage result vocabulary
from `unbill-storage` under `#[cfg(test)]` only.
The durable storage design lives in `unbill-storage`.

The net module re-exports symmetric channel endpoint and protocol helpers unconditionally.
The networking design lives in `unbill-symmetric-channel`.

These modules are convenience surfaces.
They should not become separate sources of storage or networking design.

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [unbill-console](unbill-console.md)
- core.belongs (from): (none)

> **Sirno generated links end.**
