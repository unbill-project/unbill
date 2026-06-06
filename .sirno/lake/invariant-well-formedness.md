---
core.name: Invariant Well Formedness
core.desc: Bill totals and shares are non-negative; every share is attributed to an actual participant; the payer is a known member.
core.category:
  - core.concept
core.belongs:
  - formal-invariants
core.refines:
  - data-model
---

Well-formedness.

Bill totals and shares are non-negative.
Every share is attributed to an actual participant of the bill.
The payer is a known member of the ledger.

These are preconditions for the other invariants.
Conservation and split completeness assume well-formed inputs.
