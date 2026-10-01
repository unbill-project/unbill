---
core.name: Settlement
core.desc: The deterministic integer-cent algorithm for suggested transfers.
core.category:
  - core.concept
core.belongs:
  - unbill
  - unbill-console
core.refines:
  - data-model
---

Settlement turns effective bills into suggested transfers between users.
It operates on integer cents only.

```mermaid
flowchart LR
    Ledger["Ledger"]
    Balances["Per-user balance map"]
    Transactions["Settlement transactions"]

    Ledger --> Balances
    Balances --> Transactions
```

For each effective bill,
the algorithm splits payer shares and payee shares.
Payer amounts are added to a per-user balance map.
Payee amounts are subtracted.
Positive balances mean the system owes that user.
Negative balances mean that user owes the system.

Share splitting floors each proportional amount,
then distributes the remainder one cent at a time to consecutive participants.
The starting index is derived from a fixed FNV-1a hash of the bill ID bytes modulo the share count.
This makes every peer arrive at the same cent allocation.

The console's bill split calculation validates both sides before invoking
the verified algorithm and returns payer and payee allocations together.
The console method and the pure settlement function share the name
`calculate_bill_split`; the settlement function expects validated numerical inputs.
The total must lie between zero and `i32::MAX` cents, each side must be nonempty
and have at most `i32::MAX` entries, and every weight must be positive.
Those bounds keep the verified products and weight totals within `i64`.
The Rust bridge normalizes the rounding index before the verified call
and does not sum weights in a narrower integer type.
Each side preserves input order and independently sums to the bill total.

Reduction partitions balances into creditors and debtors.
Both sides are sorted by amount descending and user ID ascending.
The largest creditor and largest debtor are matched,
the transaction amount is the smaller remaining balance,
and exhausted entries are removed.
The loop stops when all balances are zero.

The result minimizes transfer count.
It does not attempt narrative fairness or payment-order preferences.

The service layer owns multi-ledger aggregation and per-user filtering.
The settlement module owns balance math and reduction.
