---
core.name: Invariant Settlement Balance
core.desc: The computed transfer set leaves every member's net position unchanged; applying all transfers drives every balance to 0.
core.category:
  - core.concept
core.belongs:
  - formal-invariants
core.refines:
  - settlement
---

Settlement is balance-preserving.

The computed "who owes whom" transfer set leaves every member's net position unchanged,
and applying all suggested transfers drives every balance to 0.

Corollary: total paid out by debtors equals total received by creditors (no phantom money).

This is the end-to-end correctness property of `compute_from_balances`:
the greedy creditor-debtor matching loop must fully exhaust all balances.
