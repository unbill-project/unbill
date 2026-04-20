# settlement — Implementation

`accumulate_balances` walks users and bills to build a balance map. `split_shares` converts a share list and a total into deterministic per-user cent allocations; it is called once for payers (credit side) and once for payees (debit side) per bill. `compute_from_balances` then pairs the largest debtor with the largest creditor until all balances are cleared.

The module is pure and self-contained, which makes it the easiest part of the core to verify with focused tests and worked examples.
