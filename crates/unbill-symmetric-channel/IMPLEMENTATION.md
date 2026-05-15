# unbill-network — Implementation

## Layout

- `protocol.rs` — ALPN constants (`unbill/sync/v1`, `unbill/join/v1`), CBOR message types (`SyncFrame`, `JoinRequest`, `JoinReply`), and length-prefixed framing (`read_msg`, `write_msg`)
- `sync.rs` — symmetric Automerge sync loop over abstract async streams; operates on `AsyncRead + AsyncWrite` with no Iroh dependency
- `join.rs` — `run_join_host` and `run_join_requester`: token validation, device append, snapshot transfer; also operates on abstract streams
- `endpoint.rs` — `UnbillEndpoint` wraps `iroh::Endpoint`, binds with the device secret key via the N0 preset, and dispatches incoming connections by ALPN to the sync or join handlers
- `node_id_ext.rs` — conversion traits (`NodeIdExt`, `SecretKeyExt`, `IrohSecretKeyExt`, `EndpointIdExt`) between unbill model types and Iroh types

## Runtime

`UnbillEndpoint` owns the Iroh endpoint and exposes two entry points: `sync_once` dials a peer and runs the sync session; `accept_loop` waits for incoming connections and dispatches each one by ALPN. Sessions load the relevant ledgers from the store, apply remote changes, save touched docs, and broadcast `ServiceEvent` on the provided channel.

## Testing

Protocol tests use in-process streams instead of real network endpoints. Coverage focuses on framing round-trips, sync convergence between two in-process peers, authorization filtering, and join success and failure paths.
