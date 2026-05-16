# unbill-asymmetric-channel

Defines the `AsymChannel` trait — the contract of the asymmetric channel between a device and a console — and provides concrete implementations for each deployment topology.

## The trait

`AsymChannel` exposes three planes:

| Plane | Methods |
|---|---|
| Control | `create_invitation`, `join_ledger`, `trigger_peer_sync` |
| Data | `asym_sync` — one Automerge sync round per call |
| Subscription | `subscribe_to_server` — broadcast receiver of `AsymChannelEvent` |

`AsymChannelEvent` currently carries `LedgerUpdated { ledger_id }`. The device pushes this whenever its ledger state changes so consoles can pull a fresh sync.

## Implementations

| Feature flag | Type | Transport | Use case |
|---|---|---|---|
| `local` | `LocalAsymChannel` | In-process | Tauri, TUI, CLI on same machine as device |
| `rpc` | `RpcAsymChannel` / `rpc::serve` | tarpc over Unix local socket | Local two-process: daemon + separate console |
| `http` | `HttpAsymChannel` | HTTP REST | Remote access; browser web frontend |

All three implementations are interchangeable from the console's perspective. The console selects an implementation at construction time based on its deployment topology.

## Rules

- the trait is the only coupling point between the console library and the device
- implementations live in this crate; the console library depends on the trait, not the implementations
- `subscribe_to_server` returns a `broadcast::Receiver`; the console polls it and initiates an `asym_sync` round when it receives `LedgerUpdated`
