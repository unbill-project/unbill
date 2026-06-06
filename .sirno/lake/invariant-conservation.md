---
core.name: Invariant Conservation
core.desc: At every derived state, the sum of all members' net balances equals exactly 0. No operation creates or destroys money.
core.category:
  - core.concept
core.belongs:
  - formal-invariants
core.refines:
  - settlement
---

Conservation (zero-sum ledger).

At every derived state, the sum of all members' net balances equals exactly 0.
No operation creates or destroys money.

Formal statement: `sum(balance[p] for p in members) == 0`

This must hold after processing any sequence of valid bills and amendments.
It follows from split completeness applied symmetrically to payer and payee sides.
