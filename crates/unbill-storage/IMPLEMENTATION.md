# unbill-storage — Implementation

## Modules

- `store` — `LedgerStore` trait and `StorageResult` type alias
- `doc` — `LedgerDoc`: wraps `automerge::AutoCommit`, exposes typed read/write operations, and broadcasts `ChangeEvent` (`LocalWrite` or `RemoteApplied`) to listeners
- `ops` — low-level Automerge map/list operations used by `LedgerDoc`
- `device_meta` — typed helpers (`load_device_labels`, `save_device_labels`, `load_pending_invitations`, `save_pending_invitations`) that serialize structured values as JSON blobs under well-known device metadata keys

## Dependencies

Depends on `unbill-model` and `unbill-event`. Store implementations live in separate crates (`unbill-store-fs`, `unbill-store-memory`, `unbill-store-http`).

## Testing

`LedgerDoc` and `ops` are unit-tested in place. Store implementations are tested in their own crates using the shared contract.
