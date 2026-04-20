# unbill-core — Implementation Notes

## Dependencies

| Crate | Why |
|-------|-----|
| `automerge` | CRDT engine |
| `autosurgeon` | Ergonomic struct ↔ Automerge mapping |
| `iroh` | P2P transport; `iroh::NodeId` used in the model |
| `tokio` | Async runtime; `tokio::fs` for flat-file storage |
| `ulid` | `Ulid` ID generation |
| `iso_currency` | ISO 4217 currency enum for the `Currency` newtype |
| `rand` | `OsRng` for cryptographically secure `InviteToken` generation |
| `dashmap` | Concurrent ledger map in `UnbillService` |
| `directories` | Platform data directory resolution |
| `serde` / `serde_json` | `meta.json` serialization |
| `ciborium` | CBOR wire framing for the sync protocol |
| `thiserror` / `anyhow` | Error types |
| `tracing` | Structured logging |

## Testing strategy

- Unit tests in each module, using `InMemoryStore` for any test with storage dependencies.
- CRDT convergence: `proptest` fuzzes arbitrary operation interleavings; two docs diverge then merge; assert equal final state.
- Sync protocol: in-process channel pairs simulate the network — no real Iroh endpoints.
- Settlement properties: total owed equals total paid; transaction count is at most n−1.
- Storage round-trips: save → reload → assert identical bytes; compact → reload → assert identical.
