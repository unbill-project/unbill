# net — Implementation

## Layout

- `protocol.rs` — ALPN constants, CBOR message types, and length-prefixed framing
- `sync.rs` — symmetric Automerge sync loop over abstract async streams
- `join.rs` — host and requester flows for device join
- `user.rs` — host and requester flows for saved-user transfer
- `endpoint.rs` — Iroh endpoint setup, ALPN dispatch, inbound accept loop, outbound sync

## Runtime

`UnbillEndpoint` owns the device key and dispatches each connection by ALPN. Sessions load the relevant ledgers, apply remote changes, save touched docs, and emit service events.

## Testing

Protocol tests use in-process streams instead of real network endpoints. Coverage focuses on sync convergence, authorization behavior, join success or failure, and saved-user transfer.
