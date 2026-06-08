---
core.desc: The computed transfer set leaves every member's net position unchanged; applying all transfers drives every balance to 0.
core.name: Invariant Settlement Balance
meta:
  frozen:
    - reviewed
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

## Verification status

Fully verified with Verus. 0 assumes.

Proved properties:
- Conservation: `transaction_sum(transactions) == positive_sum(balances)`.
- Positivity: every `transaction.amount_cents > 0`.

Precondition: `seq_sum(balances) == 0` (credits cancel debts), values bounded.

## Proof structure

The verified function `compute_from_balances` in `unbill-console-verified/src/balance/exec.rs`
implements the greedy creditor-debtor matching and proves conservation.

Two loops:
1. Partition loop — separates balances into creditor/debtor lists.
   Invariant: `cred_sum - debt_sum == seq_sum(balances[0..i])`.
   At end: `seq_sum(balances) == 0` gives `cred_sum == debt_sum`.

2. Greedy loop — matches creditors to debtors, emitting transactions.
   Invariant: `emitted_sum + range_sum(remaining_credits) == original_total`
   (same for debts). When loop exits, either all credits or all debts
   are consumed. Since totals match, the other side is also zero.

The production bridge (`verified_bridge.rs`) converts between
verified `Transaction` (Vec<u8> user IDs) and production `Transaction` (UserId).
