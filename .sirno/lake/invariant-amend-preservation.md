---
core.name: Invariant Amend Preservation
core.desc: Applying any amendment to the append-only log yields a well-formed state where split completeness and conservation still hold.
core.category:
  - core.concept
core.belongs:
  - formal-invariants
core.refines:
  - bill-amendment
---

Amend preserves invariants.

Applying any amendment to the append-only log yields a well-formed state
in which split completeness and conservation still hold.

A bill's "current view" is a deterministic function of its amend chain
(latest-wins per field).

This means the superseded-bill filtering in `compute_settlement`
correctly excludes amended bills and includes only effective ones,
preserving the zero-sum property.
