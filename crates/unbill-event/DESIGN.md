# unbill-event

A single `ServiceEvent` enum that carries runtime notifications from the network and sync layers to the application layer.

## Events

- `LedgerUpdated { ledger_id }` — a ledger was saved; emitted by the store on every successful `save_ledger` regardless of write path (local operation, remote sync, or join)
- `PeerConnected { ledger_id, peer }` — a peer opened a sync connection for a ledger
- `PeerDisconnected { ledger_id, peer }` — a peer's sync connection closed
- `SyncError { ledger_id, peer, error }` — a sync session failed

## Rules

- events are informational; no action is required on receipt
- this crate has no dependencies and no logic
