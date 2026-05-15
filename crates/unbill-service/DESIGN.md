# unbill-service

The device-side service for unbill. It implements the Device role from the architecture: owning a `LedgerStore`, running the sym channel endpoint for P2P sync, and serving the asym channel server side for connected consoles.

## Responsibilities

- Open and hold the local `LedgerStore`
- Derive the device `NodeId` from the store's secret key
- Handle asym channel requests from consoles: invitation creation, ledger join, peer sync trigger, and Automerge sync rounds
- Run the `UnbillEndpoint` accept loop to serve incoming sym channel connections from peer devices
- Broadcast `ServiceEvent` to all subscribers when ledger state changes

## Contract

`UnbillService` is not aware of the `AsymChannel` trait. It is a plain Rust struct that `LocalAsymChannel` (in `unbill-asymmetric-channel`) wraps to satisfy the trait. Other transport implementations (RPC server, HTTP server) call the same methods directly.

## Rules

- one `UnbillService` instance per device process; it holds the store and endpoint exclusively
- the sym channel endpoint is created lazily on first use if not already running
- `accept_loop` runs until the endpoint closes; it is the long-lived background task for daemon deployments
- pending invitation tokens are persisted in local device metadata, not in shared ledger state
- join URL format: `unbill://join/<ledger_id>/<host_node_id>/<token>`
