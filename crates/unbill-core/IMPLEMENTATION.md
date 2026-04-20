# unbill-core — Implementation

## Modules

- `model/` — domain structs and newtypes
- `doc/` — `LedgerDoc` over `automerge::AutoCommit`
- `storage/` — `LedgerStore`, `FsStore`, and `InMemoryStore`
- `net/` — protocol framing, sync, join, user transfer, Iroh endpoint
- `service/` — `UnbillService` orchestration and events
- `settlement/` — balance math and transaction reduction

## Runtime

Ledgers save as full snapshot bytes plus denormalized metadata. Device-local metadata stores saved users, device labels, and pending invite state.

## Testing

Unit tests live beside the modules they cover. Storage uses `InMemoryStore`, sync uses in-process streams, and settlement tests assert balance conservation and bounded transaction count.
