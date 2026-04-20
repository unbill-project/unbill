# doc — Implementation

`ledger_doc.rs` owns the `AutoCommit` instance, change broadcaster, save and load helpers, and the Automerge sync hooks. `ops.rs` performs initialization, hydration, validation, and reconciliation.

The write path follows a simple pattern: hydrate ledger, validate input, mutate the typed value, reconcile it back, then let callers decide when to persist bytes. This keeps document logic deterministic and easy to test.
