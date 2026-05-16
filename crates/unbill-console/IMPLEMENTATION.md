# unbill-console — Implementation

## Modules

- `service/` — `UnbillConsole` orchestration and events (transitional; will become `UnbillConsole`)
- `settlement/` — balance math and transaction reduction
- `conflict/` — Union-Find conflict detection over effective bills
- `storage/` — thin re-export of `LedgerStore` and `StorageResult` from `unbill-storage`
- `net/` — thin re-export of `UnbillEndpoint` and protocol helpers from `unbill-symmetric-channel` (behind the `local` feature flag)

Domain types (`unbill-model`), CRDT document (`unbill-storage::LedgerDoc`), and service events (`unbill-event`) are provided by dedicated crates and re-exported from `unbill-console` for convenience.

## Runtime

Ledgers save as full snapshot bytes plus denormalized metadata. Device-local metadata stores saved users, device labels, and pending invite state.

## Testing

Unit tests live beside the modules they cover. Storage uses `InMemoryStore`, sync uses in-process streams, and settlement tests assert balance conservation and bounded transaction count.
