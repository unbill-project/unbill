# unbill-device — Implementation

## Layout

`lib.rs` contains the entire crate: `UnbillDevice`, its `open` constructor, and all public methods. There are no sub-modules.

## Key types

- `UnbillDevice` — holds `Arc<dyn LedgerStore>`, `NodeId`, and `UnbillEndpoint`
- `ServiceEvent` — re-exported from `unbill-event`; emitted by the store on every `save_ledger`; `UnbillDevice::subscribe()` delegates to `store.subscribe()`

## Notable behaviors

- `open` calls `store.create_secret_key()` (idempotent) then derives `device_id` via `store.get_device_id()`
- `asym_sync` runs a single Automerge sync round: decode the client message, apply it to the loaded ledger doc, save if non-empty, and return the device's sync response
- `trigger_peer_sync` and `join_ledger` reuse the running endpoint if one is active; otherwise they bind an ephemeral endpoint for the operation and close it afterward
- `accept_loop` binds a persistent endpoint, stores it in `self.endpoint`, and runs `UnbillEndpoint::accept_loop_inner` until the endpoint closes

## Dependencies

- `unbill-storage` — `LedgerStore` trait, `LedgerDoc`, invitation persistence helpers
- `unbill-symmetric-channel` — `UnbillEndpoint`, `JoinRequest`
- `unbill-event` — `ServiceEvent`
- `unbill-model` — domain types

## Testing

Integration tests use `unbill-store-memory` to exercise the full `UnbillDevice` surface without touching disk or the network.
