# settlement — Implementation

`accumulate_balances` walks users and bills to build a balance map. `split_amounts` converts weighted shares into deterministic cent allocations. `compute_from_balances` then pairs the largest debtor with the largest creditor until all balances are cleared.

The module is pure and self-contained, which makes it the easiest part of the core to verify with focused tests and worked examples.
