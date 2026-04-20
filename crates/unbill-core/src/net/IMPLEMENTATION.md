# net — Implementation Notes

## Module breakdown

| File | Responsibility |
|------|---------------|
| `mod.rs` | Re-exports `UnbillEndpoint`. |
| `protocol.rs` | ALPN constants, CBOR message types for all three protocols, length-prefixed framing helpers (read/write a `u32` byte-count then a CBOR payload). |
| `sync.rs` | `run_sync_session` — drives the `Hello`/`HelloAck` handshake then the Automerge sync loop for all accepted ledgers over an already-open bidirectional stream. No Iroh dependency; takes abstract `AsyncRead + AsyncWrite`. |
| `join.rs` | `run_join_host` and `run_join_requester` — host and requester sides of `unbill/join/v1`. Host reads peer `NodeId` from the caller (passed in from the Iroh connection); requester dials the host NodeId from the invite URL. |
| `user.rs` | `run_user_host` and `run_user_requester` — host and requester sides of `unbill/user/v1`. Same pattern as join. |
| `endpoint.rs` | `UnbillEndpoint` — wraps `iroh::Endpoint`. Opens the endpoint with the device secret key. Dispatches incoming connections to the correct handler by ALPN. Exposes `sync_once(peer_node_id)` and `accept_loop()`. |

## Dependencies

| Crate | Why |
|-------|-----|
| `iroh` | QUIC transport, `NodeId`, `SecretKey`, relay |
| `ciborium` | CBOR serialization for wire messages |
| `serde` | Derive on message types |
| `tokio` | Async I/O, channels |

## Wire framing

Every message on every protocol is framed identically:

```
[ u32 big-endian length ][ CBOR-encoded message bytes ]
```

`protocol.rs` exposes `write_msg<T: Serialize>` and `read_msg<T: DeserializeOwned>` generic helpers. All protocol handlers use only these two functions.

## Sync session flow

`run_sync_session` takes:
- `our_node_id: NodeId`
- `peer_node_id: NodeId` (TLS-verified by caller)
- `ledgers: &DashMap<String, Arc<Mutex<LedgerDoc>>>`
- `stream: (impl AsyncRead, impl AsyncWrite)`

Steps:
1. Initiator sends `Hello { ledger_ids }`. Responder reads it, computes `accepted` (ledgers where peer's `NodeId` appears in `devices`), sends `HelloAck`.
2. For each accepted ledger, both sides create a fresh `automerge::sync::State`.
3. Both sides loop: call `generate_sync_message`, send `SyncMsg` if `Some`, read incoming messages and call `receive_sync_message`, send `SyncDone` when `generate_sync_message` returns `None`.
4. When all ledgers have exchanged `SyncDone`, the function returns.

The function is symmetric — the same code runs on both sides with an `is_initiator: bool` flag controlling who sends `Hello` first.

## Testing

All protocol logic is tested without real Iroh endpoints. `tokio::io::duplex` creates an in-process bidirectional stream. Two tasks each run one side of the session concurrently.

- `test_sync_converges_after_divergence` — two `LedgerDoc` instances apply independent writes, run `run_sync_session`, assert identical final state.
- `test_sync_empty_hello_ack` — `HelloAck.accepted` is empty when peer has no shared authorized ledgers; both sides close cleanly.
- `test_join_adds_device_to_ledger` — mock host validates token and returns ledger bytes; requester loads and persists them.
- `test_user_transfer_round_trip` — host sends `(user_id, display_name)`; requester receives and stores them.
