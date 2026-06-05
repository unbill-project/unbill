---
core.desc: The deterministic integer-cent algorithm for suggested transfers.
core.name: Settlement
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
then assigns the full remainder to one deterministic recipient.
The recipient index is derived from a fixed FNV-1a hash of the bill ID bytes modulo the share count.
This makes every peer arrive at the same cent allocation.

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

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [unbill](unbill.md)
  - [unbill-console](unbill-console.md)
- core.belongs (from): (none)

> **Sirno generated links end.**
