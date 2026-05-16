# settlement — Implementation

## Entry point

`compute_settlement(ledger: &Ledger) -> Settlement` is the single public entry point. It derives effective bills from `ledger.bills`, reads `ledger.currency`, and runs the two steps below.

## Step 1 — `accumulate_balances`

Iterates over effective bills. For each bill, calls `split_shares` twice — once for the payer list, once for the payee list — and folds the results into a `HashMap<Ulid, i64>`. Payer amounts are added, payee amounts are subtracted.

## Step 2 — `compute_from_balances`

Partitions the balance map into creditors (positive) and debtors (negative). Both lists are sorted by `(amount desc, user_id asc)` to guarantee a deterministic order independent of `HashMap` iteration. The greedy loop pairs the head of each list, emits a `Transaction`, reduces both balances by `min(credit, debt)`, and advances past any exhausted entry.

## `split_shares`

Takes a share list, a total in cents, and the bill ID.

1. Computes `floor((total_cents × share_weight) / total_weight)` for each entry using integer arithmetic after the initial division.
1. Computes `remainder = total_cents − sum(floored amounts)`.
1. Selects the recipient index as `fnv1a(bill_id_bytes) mod len(shares)` and adds the full remainder to that entry.

FNV-1a is used because it is simple, dependency-free, and fully deterministic. The bill ID bytes are the 16 raw bytes of the `Ulid`.

## Testing

- `split_shares`: exact division, remainder assignment, proportional weights, zero total weight.
- `compute_settlement`: balances to zero, net sum zero, at-most n−1 transactions, already-settled case.
- Determinism: same ledger input produces identical output across multiple calls.
