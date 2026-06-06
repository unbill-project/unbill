---
core.name: Invariant No Overflow
core.desc: All monetary arithmetic stays within i64; no wraparound under any sequence of valid bills.
core.category:
  - core.concept
core.belongs:
  - formal-invariants
core.refines:
  - settlement
---

No overflow.

All monetary arithmetic — bill totals, running balances, settlement amounts —
stays within i64 bounds.
No wraparound occurs under any sequence of valid bills.

This requires establishing bounds on bill totals and share weights,
or proving that the arithmetic operations used (multiply, divide, add, subtract)
cannot exceed i64::MAX or fall below i64::MIN given valid inputs.
