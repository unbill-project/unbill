# unbill — Implementation Reference

## Workspace

Mixed monorepo: Cargo workspace for Rust, pnpm workspace for JavaScript.

| Path | Role |
|------|------|
| `crates/unbill-core/` | Library: CRDT model, storage, networking, service, settlement |
| `crates/unbill-cli/` | clap-based CLI frontend |
| `crates/unbill-tauri/` | Tauri backend (desktop/mobile) |
| `apps/unbill-desktop/` | React frontend served inside the Tauri webview |

Each component has its own DESIGN.md and IMPLEMENTATION.md.

## unbill-core module structure

| Module | Responsibility |
|--------|---------------|
| `model/` | Domain types: Bill, Member, Ledger, Amendment, Share, and all newtypes |
| `doc/` | LedgerDoc — Automerge-backed in-memory ledger; read/write operations |
| `storage/` | LedgerStore trait, FsStore (flat-file), InMemoryStore (tests) |
| `net/` | Iroh endpoint, sync protocol, per-peer connection management |
| `service/` | UnbillService — top-level facade coordinating all modules |
| `settlement/` | Net balance computation, minimum cash flow algorithm |
| `error.rs` | Shared error types |

`unbill-core` is a single crate. Sub-crate splitting is deferred until there is a concrete reason (different feature flags, MSRV, separate crates.io publication, or compile-time pain).

## Domain newtypes

All identifiers and semantic primitives use newtypes to prevent misuse at compile time.

| Type | Wraps | Stored in CRDT as |
|------|-------|-------------------|
| `Ulid` | `ulid::Ulid` | 26-char canonical string |
| `Timestamp` | `i64` | i64 unix milliseconds |
| `Currency` | `iso_currency::Currency` | 3-char ISO 4217 code |
| `NodeId` | `iroh::NodeId` | Iroh base32-hex string |
| `InviteToken` | `String` | Never persisted |

## Persistence

### Directory layout

One root per device, resolved via the `dirs` crate:
- Linux: `~/.local/share/unbill/`
- macOS: `~/Library/Application Support/unbill/`
- Windows: `%APPDATA%\unbill\`

Under that root:
- `device_key.bin` — Iroh SecretKey (32 raw bytes).
- `device_meta.json` — display name, locale, etc.
- `ledgers/<ledger_id>/ledger.bin` — full Automerge snapshot, rewritten on every mutation.
- `ledgers/<ledger_id>/meta.json` — denormalized metadata for fast listing without loading Automerge bytes.

### Write model

Every mutation calls `doc.save()` and atomically overwrites `ledger.bin` (write to `*.tmp`, rename over target). Ledger data is small enough that a full rewrite on each change is cheaper than the complexity of incremental append + compaction.

All ledgers are loaded eagerly at `UnbillService::open`. Ledger counts and sizes are small enough that this is never a problem.

### LedgerStore trait

Six async methods: `save_ledger_meta`, `list_ledgers`, `load_ledger_bytes`, `save_ledger_bytes`, `delete_ledger`, `load_device_meta`, `save_device_meta`. Two implementations: `FsStore` (production) and `InMemoryStore` (tests).

## Networking

### Transport

Iroh provides QUIC + TLS 1.3 P2P transport with built-in NAT traversal and relay fallback. Device identity is an Ed25519 `SecretKey` generated at first launch and stored in `device_key.bin`. The derived `NodeId` is the device's stable identity — used in the ledger's member device list for authorization.

### Sync protocol layers

1. **Discovery** — Iroh's built-in mechanisms: mDNS on LAN, DNS for WAN, optional DHT.
2. **Transport** — QUIC connection with ALPN `unbill/sync/v1`. One connection per peer; one stream per ledger (multiple ledgers multiplex on the same connection).
3. **Application** — Handshake (Hello / HelloAck), optional join request for new members, then Automerge's sync protocol. Frames are length-prefixed CBOR.

### Authorization

Incoming connections are accepted only if the peer's NodeId appears in the ledger's device list. Devices are authorized at the ledger level — there is no per-member device binding.

### Connection lifecycle

Per (ledger_id, peer_node_id): Disconnected → Connecting → Handshaking → InitialSync → Idle ↔ Syncing. Errors trigger exponential backoff capped at 60 seconds.

No per-peer sync state is persisted. On reconnect, both sides exchange current heads and send only missing changes — one extra round-trip, imperceptible at human scale.

### Invitation flow (M4)

1. Any existing authorized device generates an `InviteToken` via `OsRng`, held only in `UnbillService` memory.
2. Token is delivered out-of-band (QR code, message) as part of a join URL containing the inviting device's `NodeId`.
3. The joining device connects, presents the token. The token is validated and consumed. The joining device's `NodeId` is appended to the ledger's device list (not to any member record). Full ledger state flows during the immediately following sync.
4. Members (named participants) are managed separately from devices. A joining user may already have a member record, or may create one after joining.

## Settlement algorithm

Net balance per user = total paid − total owed. Balances across all users sum to zero.

Minimum cash flow via greedy: pair the largest debtor with the largest creditor, transfer the lesser of their absolute balances, remove the satisfied party, repeat. Produces at most n−1 transactions for n participants.

All arithmetic in integer cents. Rounding is display-only. Remainder cents from awkward divisions are assigned to earliest participants by user ID order — deterministic, so all devices agree.

## Schema versioning

The Ledger struct carries `schema_version`. On load, any version below current runs forward-only migration functions before use. The sync handshake advertises the minimum supported schema version; peers with incompatible versions are rejected. Wire protocol breaking changes bump the ALPN version suffix.

## Key dependencies

| Crate | Role |
|-------|------|
| `automerge` | CRDT engine |
| `autosurgeon` | Struct ↔ Automerge mapping |
| `iroh` | P2P transport and device identity |
| `tokio` | Async runtime and file I/O |
| `ulid` | Ulid ID generation |
| `iso_currency` | ISO 4217 currency enum |
| `rand` | `OsRng` for `InviteToken` generation |
| `serde` / `serde_json` | `meta.json` serialization |
| `ciborium` | CBOR wire framing |
| `dashmap` | Concurrent ledger map in `UnbillService` |
| `dirs` | Platform data directory resolution |
| `thiserror` / `anyhow` | Error types |
| `tracing` | Structured logging |
| `clap` | CLI argument parsing (`unbill-cli` only) |
| `tauri` | Desktop app shell (`unbill-tauri` only) |
| `proptest` | Property-based testing |
