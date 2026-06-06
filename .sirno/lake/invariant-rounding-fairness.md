---
core.name: Invariant Rounding Fairness
core.desc: When a total is not evenly divisible, the remainder is fully distributed and every share is within one cent of the ideal proportion.
core.category:
  - core.concept
core.belongs:
  - formal-invariants
core.refines:
  - settlement
---

Rounding exactness and fairness.

When a total is not evenly divisible, the remainder is fully distributed
and every share is within one minor unit (1 cent) of `total / n`.

Split completeness holds exactly:
e.g. 100 cents split 3 ways produces 34/33/33, never 33/33/33 (which silently loses a cent).

The remainder recipient is selected deterministically via FNV-1a hash of the bill ID,
so all peers agree on who absorbs the rounding cent.
