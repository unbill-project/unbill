---
core.name: Invariant Derivation Determinism
core.desc: Given the same ordered set of bills and amendments, balance derivation returns the same result on every replica.
core.category:
  - core.concept
core.belongs:
  - formal-invariants
core.refines:
  - settlement
  - sync-behavior
---

Determinism of derivation (the CRDT bridge).

Given the same ordered set of bills and amendments,
balance derivation returns the same result.

Order-independence across replicas
(i.e. any two replicas with the same op set converge)
is the CRDT property, deferred to the Automerge layer.

What this invariant covers is the narrower claim:
`split_shares`, `compute_settlement`, and `compute_from_balances`
are pure deterministic functions of their inputs.
No hidden state, no randomness, no floating-point non-determinism.
