---
core.name: Formal Invariants
core.desc: Formal properties the ledger system must satisfy, targeted for Creusot verification.
core.category:
  - core.concept
core.belongs:
  - unbill
core.refines:
  - settlement
  - unbill-console
---

These are the invariants that the unbill ledger system must uphold.
Each is a candidate for unbounded formal verification via Creusot contracts.
The properties span bill splitting, balance derivation, settlement, and amendment.

Verification status per invariant is tracked in the individual entries.
CRDT convergence (order-independence across replicas) is deferred to the Automerge layer.
