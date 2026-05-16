# conflict — Implementation

The module is pure and takes no I/O. Its only input is the full bill list from a `LedgerDoc`; its output is a `Vec<ConflictGroup>`.

## Structure

`detect` builds a Union-Find over every bill ID in the full bill list, unions each bill with the IDs in its `prev` list, then groups all bills by root. For each group, effective bills become `conflicting` and non-effective bills become `ancestors`. Groups with fewer than two effective members are discarded.

## Types

- `ConflictGroup` — two fields sharing the same Union-Find root:
  - `conflicting: Vec<Bill>` — effective bills in the component; always at least two
  - `ancestors: Vec<Bill>` — non-effective bills in the component; the superseded history leading to the conflict

## Testing

Tests assert the following behaviors:

- A ledger with no amendments produces no conflict groups.
- A linear amendment chain (A → B → C) produces no conflict groups.
- Two independent amendments of the same bill produce one conflict group containing both.
- Merging a conflict group (creating D with `prev = [B, C]`) removes the conflict.
- A multi-bill `prev` that supersedes a chain produces no spurious conflicts.
- Results are deterministic regardless of bill insertion order.
