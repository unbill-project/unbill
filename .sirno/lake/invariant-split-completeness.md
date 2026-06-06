---
core.name: Invariant Split Completeness
core.desc: For every bill, the shares assigned to participants sum exactly to the bill's total.
core.category:
  - core.concept
core.belongs:
  - formal-invariants
core.refines:
  - settlement
---

Split completeness.

For every bill, the shares assigned to participants sum exactly to the bill's total.

Formal statement: `sum(split_shares(shares, total, bill_id)) == total`

This is the foundational property of `split_shares`.
The floor-then-remainder algorithm must distribute every cent.
