# unbill — Design Document

> Status: Draft (v0.1)
> Last updated: 2026-04-17
> Project codename: **unbill** (tentative; subject to renaming)

## 0. Preamble: Design-First Philosophy

**This project is design-first.** Every non-trivial module — starting with each crate in the Cargo workspace, and later including each significant submodule within `unbill-core` — **must begin with a `DESIGN.md`** in its own directory before any production code is written.

### Why design-first

1. **Forces clarity on invariants.** A distributed ledger built on CRDTs has many subtle invariants (monotonic operation logs, tombstones that cannot be resurrected carelessly, membership that must be self-certifying). These are easier to surface in prose than in code.
2. **Makes trade-offs explicit.** Every design decision here has alternatives. Writing them down prevents "we did it this way because the code already did it this way" drift.
3. **Aligns with the project's academic/research potential.** This software is plausibly a basis for systems research on P2P collaboration. The design docs double as research notes.
4. **Helps a single developer stay sane.** The author is a PhD student with many competing priorities. A written design lets you pick the project back up after weeks away without rebuilding context from scratch.

### Structure of a crate-level `DESIGN.md`

Each crate's `DESIGN.md` **must** contain the following sections:

1. **Purpose**: one paragraph on what this crate is for and what it is *not* for.
2. **Public API sketch**: the types and functions that will be exposed, with signatures.
3. **Invariants**: what properties the crate guarantees to its callers.
4. **Failure modes**: what can go wrong, what errors look like, what the caller is expected to handle.
5. **Dependencies**: on other crates (internal and external), and why each is justified.
6. **Testing strategy**: how correctness will be verified, especially for concurrent / distributed behavior.
7. **Open questions**: known unknowns, deferred decisions, things to revisit.

A `DESIGN.md` does not need to be long. Two pages is often enough. But it must exist before `src/lib.rs` gets non-trivial code.

### Rules for editing `DESIGN.md`

- When the design changes, update the doc in the **same commit** as the code change. Drift between design and implementation is worse than no doc at all.
- When a decision is overturned, **keep the old reasoning** in a "Rejected alternatives" or "History" section. Future you will want to know why something was tried and abandoned.
- When a section says "Open question," the PR that resolves it should move the answer into the main body and note the resolution.

---

## 1. What is unbill

**unbill** is a fully decentralized, peer-to-peer, offline-first bill-splitting application. Think of it as "Splitwise without servers."

- A group of friends shares a **ledger**: a history of expenses, who paid, who owes whom.
- Each participant runs unbill on their own device(s). **There is no central server.**
- Devices sync directly with each other over P2P networking (Iroh) when online; operations made while offline propagate automatically once connectivity returns.
- All state lives on users' devices. The author of this software **never sees user data**, has no backend costs, and cannot lock users into a service.

### Non-goals

- **Not a payment processor.** unbill records *who owes whom*. It does not move money. Users settle outside the app (cash, Venmo, WeChat, Zelle, bank transfer, etc.).
- **Not a general-purpose accounting tool.** No double-entry bookkeeping, no tax reports, no multi-currency hedging.
- **Not a commercial service.** The author will not sell this app, run a SaaS, or monetize user data. The project is open source and free.
- **Not a high-trust environment.** It assumes group members are friends or family who will not intentionally corrupt the ledger. We do not defend against malicious insiders in v1 (see §10).

### Target user

A person who splits expenses with a small group (roommates, couple, travel party, family) and wants:

- Data ownership (their ledger on their devices).
- No account, no login, no subscription.
- Multi-device sync (phone + laptop) without cloud.
- Works on a plane, resumes when back on WiFi.

---

## 2. Core design principles

These principles guide every design decision in the project. When in doubt, reach for the principle.

1. **Offline-first.** Every operation must work without network. Sync is opportunistic.
2. **Data lives with users.** No server is ever required to use the app. If the author disappears, existing installs keep working forever.
3. **CRDTs over consensus.** We never ask "what is the true state?" We only ask "what operations have I seen?" State is a deterministic function of operations.
4. **Append-only at the data layer.** Deletion is tombstoning. Modification is amendment-or-replacement. The ledger is an event log; the UI renders a projection.
5. **Design before code.** Every crate and significant module starts with a `DESIGN.md`.
6. **One layer of indirection per concern.** Persistence is separate from business logic. Networking is separate from persistence. UI is separate from core. If layers start leaking, that's a design smell.
7. **Pluggable where it matters, monolithic where it doesn't.** The storage backend is a trait; the CRDT implementation is not. Abstract only where a real alternative exists.
8. **Testable without a network.** Core logic must be unit-testable against in-memory fixtures. Distributed behavior is tested with simulated peers, not real Iroh endpoints, wherever possible.
9. **Rust for the engine, anything for the UI.** The engine is a Rust library. Today we ship a CLI and a Tauri desktop app. Tomorrow could be a TUI, a web version, or a mobile app — they all consume the same library.
10. **Conservative about what goes into the CRDT.** Not every piece of state belongs in the synced document. Device-local preferences, UI state, caches — these stay out.

---

## 3. System architecture

### 3.1 High-level diagram

```
 ┌─────────────────────────────────────────────────────────────────┐
 │                        User devices                              │
 │   (each runs its own instance; no server anywhere)               │
 └─────────────────────────────────────────────────────────────────┘

   Alice's laptop                   Bob's phone
 ┌────────────────────┐           ┌────────────────────┐
 │  Tauri desktop     │           │  Tauri mobile      │
 │  React UI          │           │  React UI          │
 │    ▲               │           │    ▲               │
 │    │ IPC           │           │    │ IPC           │
 │    ▼               │           │    ▼               │
 │  unbill-core (lib) │◄─── P2P ──►│  unbill-core (lib) │
 │  - CRDT doc        │   Iroh    │  - CRDT doc        │
 │  - Storage         │   QUIC    │  - Storage         │
 │  - Sync engine     │           │  - Sync engine     │
 └────────────────────┘           └────────────────────┘
         │                                 │
         ▼                                 ▼
   SQLite on disk                   SQLite on disk
```

Optional relay nodes (provided by the Iroh project, or self-hosted) participate only when two peers cannot establish a direct connection due to NAT. They forward encrypted bytes and cannot see ledger contents.

### 3.2 Workspace layout

The repository is a mixed monorepo: a **Cargo workspace** for Rust, and a **pnpm workspace** for JavaScript frontends.

```
unbill/
├── Cargo.toml                # workspace root
├── Cargo.lock
├── package.json              # pnpm root
├── pnpm-workspace.yaml
├── README.md
├── DESIGN.md                 # this file (project-level design)
├── LICENSE
├── .gitignore
│
├── crates/
│   ├── unbill-core/          # The library crate (THE project)
│   │   ├── Cargo.toml
│   │   ├── DESIGN.md         # MUST exist before implementation
│   │   ├── README.md
│   │   └── src/
│   │       └── lib.rs
│   │
│   ├── unbill-cli/           # CLI frontend
│   │   ├── Cargo.toml
│   │   ├── DESIGN.md         # MUST exist
│   │   └── src/
│   │       └── main.rs
│   │
│   └── unbill-tauri/         # Tauri desktop/mobile backend
│       ├── Cargo.toml
│       ├── DESIGN.md         # MUST exist
│       ├── tauri.conf.json
│       ├── build.rs
│       ├── icons/
│       └── src/
│           └── main.rs
│
└── apps/
    └── unbill-desktop/       # React frontend for Tauri
        ├── package.json
        ├── DESIGN.md         # MUST exist (UI component design)
        ├── vite.config.ts
        ├── index.html
        └── src/
            ├── main.tsx
            └── App.tsx
```

### 3.3 Crate responsibilities

| Crate | Responsibility | Depends on |
|-------|---------------|------------|
| `unbill-core` | **Everything that is the actual product.** CRDT document model, persistence, networking, sync protocol, business logic (bill splitting, settlement computation). This is the library that defines what unbill *is*. | Third-party only. |
| `unbill-cli` | A thin command-line frontend. Creates ledgers, adds bills, runs the sync daemon, exports/imports. Useful for dogfooding, automated testing, and users who prefer the terminal. | `unbill-core`. |
| `unbill-tauri` | Thin Tauri backend. Exposes `unbill-core` as Tauri commands and events so the React UI can drive it. Contains no business logic. | `unbill-core`, `tauri`. |

**Intentionally a single core crate (for now).** An earlier draft split the core into `unbill-doc`, `unbill-storage`, `unbill-net`, and `unbill-service` crates. That was premature. We will revisit splitting after these conditions are met:

- The internal module boundaries inside `unbill-core` have stabilized over multiple releases.
- There is a concrete reason to split, e.g. one layer needs different feature flags, a different MSRV, or to be published separately to crates.io.
- Compilation times become a meaningful developer pain point.

Until then, `unbill-core` is one crate with clean internal modules. Module boundaries are enforced by code review and the crate-level `DESIGN.md`, not by the crate boundary.

### 3.4 Internal modules of `unbill-core`

Though everything lives in one crate, the internal structure mirrors the eventual split:

```
crates/unbill-core/src/
├── lib.rs              # re-exports the public API, nothing else
├── model/              # domain types: Bill, Member, Amendment, Settlement
│   ├── mod.rs
│   ├── bill.rs
│   ├── member.rs
│   └── amendment.rs
├── doc/                # Automerge integration
│   ├── mod.rs
│   ├── ledger_doc.rs   # LedgerDoc: the CRDT-backed in-memory ledger
│   └── ops.rs          # low-level Automerge operations
├── storage/            # persistence
│   ├── mod.rs
│   ├── traits.rs       # LedgerStore trait
│   ├── sqlite.rs       # SQLite implementation
│   └── memory.rs       # in-memory implementation (testing)
├── net/                # P2P networking
│   ├── mod.rs
│   ├── endpoint.rs     # Iroh endpoint lifecycle
│   ├── protocol.rs     # handshake, ALPN, message framing
│   └── sync.rs         # per-peer sync loop
├── service/            # orchestration
│   ├── mod.rs
│   └── service.rs      # UnbillService: the top-level facade
├── settlement/         # business logic: who owes whom
│   └── mod.rs
└── error.rs            # shared error types
```

Each subdirectory (`model/`, `doc/`, `storage/`, `net/`, `service/`, `settlement/`) will get its own `DESIGN.md` under `crates/unbill-core/src/<module>/DESIGN.md` before substantial implementation begins. These sub-design-docs can be brief — the crate-level `DESIGN.md` carries the big picture.

---

## 4. Data model

### 4.1 A ledger is an Automerge document

A **ledger** — "our household expenses," "the Iceland trip," "Alice and Bob's grocery split" — corresponds 1:1 to a single Automerge document. A user may have many ledgers on their device, each independent.

Why 1 ledger = 1 doc (not one giant doc for all ledgers):

- **Access control granularity.** Different ledgers have different members. Sync protocol only ships ops to members of that ledger.
- **Blast radius.** A bug or corruption in one ledger doesn't affect others.
- **Performance.** Large history doesn't slow down unrelated ledgers.
- **Deletion.** Dropping a ledger is `DELETE FROM ledgers WHERE id = ?`. Clean.

### 4.2 Ledger schema

Using `autosurgeon` for ergonomic CRDT ↔ Rust struct mapping. Schema is intentionally small; additions require a migration plan.

```rust
#[derive(Clone, Reconcile, Hydrate)]
pub struct Ledger {
    pub ledger_id: String,      // ULID, generated at creation, never changes
    pub name: String,           // human-readable, editable
    pub currency: String,       // ISO 4217 code, e.g. "USD", "CNY"
    pub created_at: i64,        // unix millis
    pub members: Vec<Member>,
    pub bills: Vec<Bill>,
    pub invitations: Vec<Invitation>,
}

#[derive(Clone, Reconcile, Hydrate)]
pub struct Member {
    pub user_id: String,        // ULID; stable across devices
    pub display_name: String,
    pub devices: Vec<Device>,   // all NodeIds belonging to this user
    pub added_at: i64,
    pub added_by: String,       // user_id of inviter
    pub removed: bool,          // tombstone (never true -> false in v1)
}

#[derive(Clone, Reconcile, Hydrate)]
pub struct Device {
    pub node_id: String,        // Iroh NodeId as hex
    pub label: String,          // e.g. "Alice's iPhone"
    pub added_at: i64,
}

#[derive(Clone, Reconcile, Hydrate)]
pub struct Bill {
    pub id: String,             // ULID
    pub payer_user_id: String,
    pub amount_cents: i64,      // always an integer; currency implicit from ledger
    pub description: String,
    pub participant_user_ids: Vec<String>,
    pub split_method: SplitMethod,
    pub created_at: i64,
    pub created_by_device: String,  // NodeId; for audit/debug
    pub deleted: bool,          // tombstone
    pub amendments: Vec<Amendment>,
}

#[derive(Clone, Reconcile, Hydrate)]
pub enum SplitMethod {
    Equal,
    Shares(Vec<Share>),         // proportional: [{user_id, shares: u32}, ...]
    Exact(Vec<ExactAmount>),    // exact cents per user
}

#[derive(Clone, Reconcile, Hydrate)]
pub struct Amendment {
    pub id: String,             // ULID
    pub new_amount_cents: Option<i64>,
    pub new_description: Option<String>,
    pub new_participants: Option<Vec<String>>,
    pub new_split_method: Option<SplitMethod>,
    pub author_user_id: String,
    pub created_at: i64,
    pub reason: Option<String>,
}

#[derive(Clone, Reconcile, Hydrate)]
pub struct Invitation {
    pub token: String,          // random 32 bytes, hex-encoded
    pub created_by_user_id: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub used_by_node_id: Option<String>,  // None if unused; some(node_id) once consumed
}
```

### 4.3 Invariants

These are enforced at the service layer and tested exhaustively:

1. **Bill IDs are ULIDs, globally unique, never reused.** Generated at creation time on the originating device.
2. **Bills are append-only.** A bill, once added, is never removed from `bills`. It may gain `deleted: true` (tombstone) or accumulate `amendments`. Real deletion from the vector is forbidden (violates merge semantics; see §4.5).
3. **Member IDs are stable.** A user's `user_id` never changes. Adding a device appends to `member.devices`.
4. **The payer and all participants of a bill must be members of the ledger at the time the bill was created.** Not enforced on load (a departed member's old bills remain valid), only at creation time.
5. **`amount_cents` is non-negative.** Refunds are modeled as separate bills with reversed payer/participant roles, not negative amounts.
6. **The ledger's currency is fixed at creation.** Multi-currency is a future concern.

### 4.4 Effective view

The UI never shows raw `Bill`s directly; it shows **`EffectiveBill`**, computed by applying amendments:

```rust
pub struct EffectiveBill {
    pub id: String,
    pub payer_user_id: String,
    pub amount_cents: i64,
    pub description: String,
    pub participants: Vec<String>,
    pub split_method: SplitMethod,
    pub was_amended: bool,
    pub is_deleted: bool,
    pub last_modified_at: i64,
    pub history: Vec<AmendmentSummary>,  // for "show history" UI
}
```

`EffectiveBill::from(bill)` applies amendments in `created_at` order; later amendments overwrite earlier ones at the field level. Ties in `created_at` are broken by `amendment.id` (lexical).

### 4.5 Why amendments (and why append-only)

This is the most subtle design decision in the project. It deserves a full explanation so future-us doesn't "simplify" it without understanding.

**Automerge resolves concurrent field writes via last-writer-wins at the field level.** If Alice edits `bill.amount_cents` to 150 and Bob concurrently edits it to 200, after merge, one of those values wins (deterministically but arbitrarily from the user's perspective). The other is lost from the rendered view (still present in history).

**This is unacceptable for money.** A user whose edit silently vanishes has been *lied to by the system*.

The amendment model solves three problems simultaneously:

1. **Semantic atomicity.** A user's edit changes `amount` *and* `description` *and* `participants` together. Those belong in one record. LWW on individual fields could cross-pollinate between concurrent edits, producing a Frankenstein bill neither user proposed.
2. **Auditability.** Every edit has an author, a timestamp, and optionally a reason. This lives in the data model, not dug up from Automerge history.
3. **Conflict visibility.** When two amendments land concurrently, the UI can say "Alice and Bob both modified this bill at roughly the same time; current display reflects Bob's version. View Alice's?" Silent resolution would hide the disagreement.

**Why not simply edit `bill` fields and accept LWW?** Because users whose edits silently disappear will lose trust in the ledger, and rebuilding trust in a financial tool is hard.

**Why not fully immutable bills (edit = delete + recreate)?** Because external references (settlements, screenshots, conversation history: "hey, about bill X...") would break. And a "modified" bill loses its identity with the original, which is exactly what users want preserved.

### 4.6 Rejected alternatives

- **Single global document containing all ledgers.** Rejected: access control becomes impossible; sync fanout is wrong (Bob in the roommate group doesn't need to hear about my family ledger's changes); deletion of a ledger means retaining garbage forever.
- **One Automerge doc per bill.** Rejected: settlement requires aggregating bills; no cross-document queries; sync state per document explodes; atomicity of "add bill" and "amend bill" is lost.
- **Edit bills in place, rely on `get_all` for conflict detection.** Rejected: pushes conflict handling into every UI read path; no first-class author/timestamp metadata; cross-field incoherence (see §4.5).
- **Event-sourcing style (append events, compute state from scratch each time).** Rejected: effectively what we have, but without the benefit of Automerge managing identity and concurrent merges for us.

---

## 5. Persistence

### 5.1 What is persisted

- **Ledger documents.** The Automerge byte representation, with periodic snapshots and incremental op logs.
- **Sync state.** For each `(ledger_id, peer_node_id)` pair, the sync protocol's persistent state (Bloom filter of known changes, last-seen heads).
- **Device identity.** The Iroh `SecretKey` used to derive this device's `NodeId`.
- **Device profile.** The local user's chosen `display_name`, preferred language, etc. — things that are *about this install* rather than *shared in the ledger*.

### 5.2 Storage layout

One SQLite database per data directory. Schema:

```sql
-- Ledger base state
CREATE TABLE ledgers (
    ledger_id       TEXT PRIMARY KEY,
    name            TEXT NOT NULL,          -- denormalized for list views
    snapshot        BLOB NOT NULL,          -- Automerge.save() output
    snapshot_heads  TEXT NOT NULL,          -- JSON array of heads at snapshot time
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

-- Append-only incremental changes since last snapshot
CREATE TABLE ledger_incremental (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ledger_id   TEXT NOT NULL,
    bytes       BLOB NOT NULL,              -- output of save_incremental()
    added_at    INTEGER NOT NULL,
    FOREIGN KEY (ledger_id) REFERENCES ledgers(ledger_id) ON DELETE CASCADE
);
CREATE INDEX idx_incremental_ledger ON ledger_incremental(ledger_id, id);

-- Per-peer sync protocol state
CREATE TABLE sync_states (
    ledger_id       TEXT NOT NULL,
    peer_node_id    TEXT NOT NULL,
    state_bytes     BLOB NOT NULL,          -- automerge::sync::State::encode()
    updated_at      INTEGER NOT NULL,
    PRIMARY KEY (ledger_id, peer_node_id)
);

-- Device-wide singletons: device_key, device_profile, etc.
CREATE TABLE device_meta (
    key     TEXT PRIMARY KEY,
    value   BLOB NOT NULL
);
```

### 5.3 Snapshot + incremental strategy

Every write to a `LedgerDoc` appends the result of `save_incremental()` to `ledger_incremental`. Loading a ledger reads the snapshot, then applies all incrementals in insertion order.

Compaction is triggered when:

- `ledger_incremental` for a ledger grows past **N rows** (default: 256), or
- Total incremental bytes exceed **M kilobytes** (default: 512), or
- Manual compaction command is issued.

Compaction is transactional:
1. Load full `LedgerDoc` from snapshot + all incrementals.
2. Compute new snapshot with `doc.save()`.
3. In a single SQLite transaction: update `ledgers.snapshot` and `snapshot_heads`, delete all rows in `ledger_incremental` for this ledger.

If compaction is interrupted, state is unchanged (the transaction either commits entirely or not at all).

### 5.4 Abstraction: the `LedgerStore` trait

```rust
#[async_trait]
pub trait LedgerStore: Send + Sync {
    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>>;
    async fn load_ledger_bytes(&self, ledger_id: &str) -> Result<LoadedBytes>;
    async fn append_incremental(&self, ledger_id: &str, bytes: &[u8]) -> Result<()>;
    async fn compact(&self, ledger_id: &str, new_snapshot: &[u8], heads: &[ChangeHash]) -> Result<()>;
    async fn delete_ledger(&self, ledger_id: &str) -> Result<()>;

    async fn load_sync_state(&self, ledger_id: &str, peer: &str) -> Result<Option<Vec<u8>>>;
    async fn save_sync_state(&self, ledger_id: &str, peer: &str, bytes: &[u8]) -> Result<()>;

    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn save_device_meta(&self, key: &str, value: &[u8]) -> Result<()>;
}

pub struct LoadedBytes {
    pub snapshot: Vec<u8>,
    pub incrementals: Vec<Vec<u8>>,
}
```

Two implementations:

- **`SqliteStore`**: the real thing. Uses `rusqlite` (synchronous) wrapped in `tokio::task::spawn_blocking`, or `sqlx` for native async. Decision deferred to the storage sub-module's `DESIGN.md`.
- **`InMemoryStore`**: a `HashMap`-backed implementation for unit tests. Same trait, zero I/O.

### 5.5 Data directory

Platform-appropriate, via the `dirs` crate:

- Linux: `~/.local/share/unbill/`
- macOS: `~/Library/Application Support/unbill/`
- Windows: `%APPDATA%\unbill\`

Single SQLite file: `unbill.db`. Device key: `device_key.bin`.

---

## 6. Networking and sync protocol

### 6.1 Transport: Iroh

We use [Iroh](https://iroh.computer) as the P2P transport. Rationale:

- **NodeId-as-identity.** Ed25519 public keys serve as device identifiers. No PKI, no account system, no username conflicts.
- **End-to-end encryption** via QUIC+TLS 1.3, built in.
- **NAT traversal** with automatic fallback to relays. We start with Iroh's free public relays; self-hosting is easy if we outgrow them.
- **Pure Rust library.** Compiles on every platform Tauri targets, including iOS and Android.
- **License friendly.** Apache 2.0 / MIT.

### 6.2 Device identity

On first launch, `unbill-core` generates an Iroh `SecretKey` and stores it in `device_meta`. The derived `NodeId` (32-byte Ed25519 public key) is the device's stable identifier.

A single *user* may have multiple *devices*, each with its own NodeId. The ledger's `Member.devices` field tracks the mapping.

### 6.3 Group membership and authorization

**v1 policy: any member may add any new member.**

Adding a new member Bob to the ledger is a three-step dance:

1. Existing member Alice creates an `Invitation` record in the ledger (random token, expires in ~1 hour).
2. Alice transmits the invitation out-of-band to Bob: a URL of the form `unbill://join?ledger=<id>&token=<t>&bootstrap=<alice_node_id>`. Delivery is via QR code, iMessage, AirDrop, email, etc.
3. Bob's app connects to Alice using the bootstrap NodeId, presents the token during handshake. Alice's app verifies the token, appends Bob (and his initial device) to `members`, marks the invitation as used, then begins sync. Bob receives the full ledger, in which he is now listed as a member.

Once a member, Bob's device(s) can sync with any other member's device(s). Authorization on incoming connections is:

```rust
fn is_authorized(ledger: &Ledger, peer_node_id: &str) -> bool {
    ledger.members.iter()
        .filter(|m| !m.removed)
        .flat_map(|m| &m.devices)
        .any(|d| d.node_id == peer_node_id)
}
```

### 6.4 Sync protocol stack

Three logical layers:

1. **Discovery.** Finding a peer's current network address given their NodeId. Iroh's built-in discovery (mDNS on LAN, Iroh's DNS discovery for WAN, optionally Mainline DHT).
2. **Transport.** Iroh QUIC connection with ALPN `unbill/sync/v1`.
3. **Application.** A brief handshake, then Automerge's sync protocol over a bidirectional stream.

### 6.5 Application-layer protocol

After QUIC handshake, one bidirectional stream per (connection, ledger) tuple. Messages are length-prefixed (u32 BE) CBOR-encoded frames.

**Handshake**

```
Initiator -> Responder: Hello { protocol_version: u32, ledger_id: String }
Responder -> Initiator: HelloAck { accepted: bool, reason: Option<String> }
```

If `accepted: false`, the responder closes the stream. Reasons include "ledger unknown" and "not authorized."

**Optional: invitation presentation**

If the initiator is a new member completing a join:

```
Initiator -> Responder: JoinRequest { token: String, device: Device, user_profile: UserProfile }
Responder -> Initiator: JoinResponse { accepted: bool }
```

The responder validates the token (not used, not expired, exists), adds the user+device to the ledger, and only then proceeds to sync.

**Sync loop**

Both sides maintain an `automerge::sync::State`. Each side, when it has a local change or receives a message, calls `generate_sync_message()` and sends the result (if non-None). Messages are framed identically to the handshake messages.

```
loop select {
    local_change_notification => {
        if let Some(msg) = doc.generate_sync_message(&mut state) {
            send(msg).await?;
        }
    }
    incoming_message => {
        let msg = Message::decode(&bytes)?;
        doc.receive_sync_message(&mut state, msg)?;
        // receive might produce new local state that warrants sending
        if let Some(reply) = doc.generate_sync_message(&mut state) {
            send(reply).await?;
        }
        // persist state periodically (throttled)
    }
}
```

The sync state is persisted to `sync_states` at most once per 5 seconds, or after 10 apply operations, whichever comes first. Losing recent sync state is harmless: the protocol is self-healing and will re-send a small amount of redundant data.

### 6.6 Connection lifecycle

For each `(ledger_id, peer_node_id)` pair where the peer is an authorized member, a background Tokio task maintains:

```
Disconnected --[discovery succeeds]--> Connecting
Connecting  --[QUIC handshake]--> Handshaking
Handshaking --[accepted]--> InitialSync
InitialSync --[converged]--> Idle
Idle        --[local change OR inbound message]--> Syncing
Syncing     --[complete]--> Idle
* --[error or disconnect]--> Backoff (exponential, capped at 60s) --> Connecting
```

One QUIC connection per peer; one stream per ledger (multiple ledgers with the same peer multiplex).

### 6.7 Realtime change propagation

`LedgerDoc` exposes a `tokio::sync::broadcast::Sender<ChangeEvent>` on which it emits every time its state changes (either from a local write or from applying a received sync message). Peer sync tasks subscribe and use the notification to trigger outbound sync messages. The UI layer also subscribes and uses the notification to trigger re-queries.

### 6.8 What happens when everyone is offline

Nothing. Users keep making local edits. On next connection, sync protocol catches up. This is the whole point of CRDTs.

### 6.9 Non-P2P fallbacks

For scenarios where P2P is undesired or impossible (initial onboarding over a one-way channel, sneakernet, sharing a ledger over email):

- **Export to file.** `doc.save()` serialized to a `.unbill` file. Shareable via AirDrop, WeChat, email, USB.
- **Import from file.** Load bytes, `merge` into the existing ledger if present, or create new.
- **Export incremental.** Since a remembered point, useful for "I made some changes, send them to you as a file."

The CLI will expose these directly. The GUI will offer them as "Share" and "Import."

---

## 7. The service layer

`UnbillService` is the top-level facade that `unbill-cli` and `unbill-tauri` consume. It owns and coordinates everything:

```rust
pub struct UnbillService {
    store: Arc<dyn LedgerStore>,
    endpoint: iroh::Endpoint,
    ledgers: DashMap<String, Arc<RwLock<LedgerDoc>>>,
    peer_tasks: DashMap<(String, NodeId), JoinHandle<()>>,
    events: broadcast::Sender<ServiceEvent>,
}
```

### 7.1 Startup

```rust
impl UnbillService {
    pub async fn start(data_dir: &Path) -> Result<Arc<Self>> {
        // 1. Open SqliteStore.
        // 2. Load or generate device key; derive NodeId.
        // 3. Start Iroh endpoint with discovery enabled.
        // 4. Enumerate all ledgers from storage; load LedgerDoc for each (or lazily on first use; TBD).
        // 5. For each ledger, for each known peer, spawn maintain_peer_connection task.
        // 6. Spawn the accept loop that handles incoming Iroh connections.
        // 7. Return Arc<Self>.
    }
}
```

### 7.2 Public API

Coarse-grained actions that are meaningful to a frontend:

```rust
impl UnbillService {
    // Ledger lifecycle
    pub async fn create_ledger(&self, name: String, currency: String) -> Result<String>;
    pub async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>>;
    pub async fn delete_ledger(&self, ledger_id: &str) -> Result<()>;
    pub async fn export_ledger(&self, ledger_id: &str) -> Result<Vec<u8>>;
    pub async fn import_ledger(&self, bytes: &[u8]) -> Result<String>;

    // Bills
    pub async fn add_bill(&self, ledger_id: &str, input: NewBill) -> Result<String>;
    pub async fn amend_bill(&self, ledger_id: &str, bill_id: &str, input: BillAmendment) -> Result<()>;
    pub async fn delete_bill(&self, ledger_id: &str, bill_id: &str) -> Result<()>;  // tombstone
    pub async fn restore_bill(&self, ledger_id: &str, bill_id: &str) -> Result<()>;
    pub async fn list_bills(&self, ledger_id: &str) -> Result<Vec<EffectiveBill>>;

    // Members
    pub async fn invite_member(&self, ledger_id: &str, display_name: String) -> Result<InvitationInfo>;
    pub async fn accept_invitation(&self, url: &str) -> Result<String>;  // returns ledger_id
    pub async fn list_members(&self, ledger_id: &str) -> Result<Vec<Member>>;

    // Settlement
    pub async fn compute_settlement(&self, ledger_id: &str) -> Result<Settlement>;

    // Device/identity
    pub fn node_id(&self) -> NodeId;
    pub async fn list_peers(&self, ledger_id: &str) -> Result<Vec<PeerInfo>>;

    // Events
    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent>;
}

pub enum ServiceEvent {
    LedgerUpdated { ledger_id: String },
    PeerConnected { ledger_id: String, peer: NodeId },
    PeerDisconnected { ledger_id: String, peer: NodeId },
    SyncError { ledger_id: String, peer: NodeId, error: String },
}
```

This is the surface that CLI and Tauri both consume. The Tauri layer wraps each method as a `#[tauri::command]` and forwards events to JS via `emit`. The CLI wraps them in `clap` subcommands.

---

## 8. Settlement algorithm

Settlement computes "who should pay whom, how much, to square all accounts."

### 8.1 Net balance

For each user, compute net balance: sum of amounts they paid minus sum of amounts they owe. Positive = owed money; negative = owes money; sum across all users = 0 (invariant).

### 8.2 Minimum cash flow

Given net balances, suggest a minimal set of transactions that settles everyone. This is NP-hard in general, but for the small `n` typical in bill-splitting (<20 people), a greedy heuristic is essentially optimal:

1. Sort users by net balance.
2. Largest debtor pays largest creditor min(their debt, creditor's surplus).
3. Remove the satisfied party; repeat.

Produces at most `n-1` transactions for `n` people. Good enough.

### 8.3 Precision

All math in integer cents. Rounding at the very end of display only. Never round intermediate values.

For split methods with awkward divisions (e.g., $10 split 3 ways), we assign the remainder cent-by-cent to the earliest participants in `participant_user_ids` order. Deterministic, everyone's computation agrees.

---

## 9. Testing strategy

### 9.1 Unit tests

Every module in `unbill-core` has unit tests. Target: no reasonable function ships untested.

Key fixtures:

- `InMemoryStore` for storage-dependent tests.
- A mock Iroh endpoint (or direct in-process channel) for sync protocol tests — we do not want tests that depend on real networking.

### 9.2 CRDT behavior tests

The most important correctness property: **for all possible orderings of operations, all devices converge to the same state.** We test this by:

1. Create two `LedgerDoc`s with the same initial state.
2. Apply arbitrary interleavings of operations to each.
3. Cross-merge.
4. Assert equal final state.

Use `proptest` to fuzz the operation sequences.

### 9.3 Sync protocol tests

With two `LedgerDoc`s and in-process channels simulating the network:

- Happy path convergence (assert they reach the same heads in bounded rounds).
- Partition + heal (send arbitrary subset of messages, then all; assert convergence).
- State loss resilience (discard one side's `SyncState`; assert convergence still happens, just with more traffic).

### 9.4 CLI end-to-end tests

Shell scripts under `tests/e2e/` that:

1. Create temp data directories for two simulated devices.
2. Use the CLI to run realistic scenarios: create ledger, add bills on both sides, export/import bundles, verify convergence.
3. Assert final state via `--json` output.

These are the closest thing we have to "did this all actually work end to end" tests, and they run in CI.

### 9.5 Property-based tests

Heavy use of `proptest` for:

- Settlement algorithm (total owed = total paid, number of transactions ≤ n-1).
- Amendment application (idempotence, associativity, commutativity where expected).
- Serialization round-trips (`save` + `load` yields equivalent doc).

### 9.6 What we do not test (yet)

- Real network flakiness. Iroh is assumed to work; we don't exercise its failure modes in our tests.
- GUI behavior. Manual testing only until something proves tricky.
- Performance. We will add benchmarks once we have real-world ledger sizes to target.

---

## 10. Security and privacy

### 10.1 What we defend against in v1

- **Passive eavesdroppers on the network.** Handled by Iroh's E2E encryption.
- **Impersonation of a device.** NodeId verification at handshake prevents anyone without the corresponding `SecretKey` from pretending to be a given device.
- **Accidental cross-group data leakage.** Authorization check at handshake: only members of ledger L can sync ledger L.

### 10.2 What we do NOT defend against in v1

We're explicit about this so users can assess risk:

- **Malicious insiders.** Any member of a ledger can, in principle, corrupt its history (e.g., create conflicting amendments to confuse others). Because this is a "friends and family" tool, we assume members are not adversarial. Future versions may add signed amendments and cryptographic audit.
- **Device compromise.** If an attacker has read/write access to a member's device storage, they can forge arbitrary history from that member's perspective. Standard OS-level protections (disk encryption, app sandbox) are assumed.
- **Revocation.** Kicking a member (`removed: true`) stops them from receiving *future* updates, but they still have the full history up to the kick. This is information-theoretically unavoidable without re-keying.
- **Metadata privacy from relays.** Iroh relays see *which* NodeIds are talking to each other, even if not what. If NodeIds leaked, a relay operator could infer social graphs. Mitigation is out of scope for v1.

### 10.3 Telemetry policy

**Absolutely none.** The application makes no outbound connection other than:

- Iroh discovery (looking up peer addresses; necessary for function).
- Iroh relays (transport fallback; necessary for function).
- Peer-to-peer sync with explicit group members.

No analytics, no error reporting home, no update checks, no "anonymous usage stats." This is non-negotiable and core to the product's positioning.

The only "phone home" behavior that might be considered: a version check against GitHub Releases. This is off by default, opt-in, and clearly labeled.

---

## 11. Frontends

### 11.1 `unbill-cli`

A `clap`-driven command-line tool. Its `DESIGN.md` will detail the subcommand tree, but the shape is:

```
unbill init
unbill ledger create|list|show|export|import|delete
unbill bill add|list|amend|delete|restore
unbill member list|invite|join
unbill sync daemon|once|status
unbill settlement show
unbill inspect heads|history|peers
```

The CLI owns `UnbillService` and runs inside a tokio runtime. For daemons, it runs until interrupted. For one-shot commands, it starts the service, performs the action, and exits.

### 11.2 `unbill-tauri` + `unbill-desktop`

The Tauri backend is thin. Each method of `UnbillService` gets wrapped as a Tauri command. Every `ServiceEvent` gets forwarded to the frontend via `app.emit("unbill:event", &event)`.

The React frontend uses:
- **TanStack Query** for data fetching and cache.
- **A custom hook** `useUnbillEvent` that listens to Tauri events and invalidates TanStack Query caches.
- **Shadcn/ui + Tailwind** for the UI kit (preference; the frontend's own `DESIGN.md` will confirm).

Mobile (iOS/Android) support is enabled by Tauri 2 but explicitly a v2 goal. The library and desktop ship first.

---

## 12. Versioning and data migration

### 12.1 Schema versions

The ledger schema will evolve. Strategy:

- The `Ledger` struct includes a top-level `schema_version: u32` field.
- On load, if `schema_version < CURRENT`, a migration function runs over the hydrated struct before it is used.
- Migrations are forward-only. We never "downgrade" a ledger.
- When a peer on an older version receives ops it doesn't understand, it must refuse to apply them rather than silently dropping them. The sync protocol's `Hello` includes the minimum supported schema version, so version skew is caught at handshake.

### 12.2 Wire protocol versions

The `unbill/sync/v1` ALPN and the `Hello.protocol_version` field manage wire compatibility. Breaking changes bump the ALPN to `/v2`. Peers advertise both if they support both, allowing graceful rollout.

### 12.3 Semver policy

- `unbill-core` is pre-1.0. Breaking API changes are expected and versioned `0.x.0`.
- Once the CLI and GUI ship to real users, we commit to "no silent data-destroying changes" — migrations run, schema versions track.

---

## 13. Roadmap

### Milestone M0: Workspace skeleton
- Create workspace, empty crates, this `DESIGN.md`.
- Each crate has a stub `DESIGN.md`.
- `cargo build` works; `cargo test` passes trivially.

### Milestone M1: Core data model
- `unbill-core::model`: `Bill`, `Member`, `Ledger`, `Amendment`, tests.
- `unbill-core::doc`: `LedgerDoc::new`, `add_bill`, `list_bills`, `to_bytes`, `from_bytes`.
- No storage, no networking. Tests run in memory.

### Milestone M2: Persistence
- `LedgerStore` trait, `SqliteStore`, `InMemoryStore`.
- `UnbillService::start` loads existing ledgers on boot.
- CLI commands: `init`, `ledger create`, `ledger list`, `bill add`, `bill list`.
- Data survives restart.

### Milestone M3: Offline file sync
- `export_ledger` / `import_ledger` / `apply_incremental_bundle`.
- CLI commands: `ledger export`, `ledger import`.
- End-to-end shell test: two devices, AirDrop-style exchange, convergence.

### Milestone M4: P2P sync
- Iroh integration: endpoint, NodeId persistence, accept loop.
- Handshake, authorization, sync loop.
- CLI commands: `sync daemon`, `sync once`, `member invite`, `member join`.
- End-to-end test: two CLI daemons, add bills on either side, observe realtime sync.

### Milestone M5: Desktop GUI
- `unbill-tauri` backend.
- `unbill-desktop` frontend: ledger list, ledger view, add-bill form, settlement view.
- Packaged builds for macOS / Linux / Windows.

### Milestone M6+ (future)
- Amendment UI and conflict resolution UX.
- Mobile (iOS, Android) via Tauri 2 mobile.
- Multi-currency and exchange rate support.
- Categories/tags on bills.
- Settlement suggestions via payment apps (copy Venmo link, etc.).
- Formal verification of core CRDT invariants using Kani or Loom.

---

## 14. Open questions

Living list. Each should be resolved in the appropriate module's `DESIGN.md` before that module is implemented.

1. **sqlx vs rusqlite.** sqlx is native async but adds build complexity. rusqlite is simpler but needs `spawn_blocking`. Decision deferred to `storage/DESIGN.md`.
2. **Lazy vs eager ledger loading at startup.** Load all on boot (simple, memory-hungry for many ledgers) or lazily (complex, necessary if users have hundreds of ledgers)?
3. **Invitation token storage.** In the ledger (so all members can see pending invites) or in device-local state (so they're private)?
4. **Multi-device onboarding.** How does Alice add her second device? Copy key file? QR code between her own devices? Design needs UX thinking.
5. **Notification strategy on mobile.** iOS backgrounding kills long-lived connections. Do we need silent push, and if so, whose infrastructure?
6. **Backup and restore.** The "user owns their data" promise implies losing a phone = losing ledgers (unless synced to another device). Should we offer explicit device backup to e.g. iCloud Drive?
7. **Name.** "unbill" is a placeholder. Good candidates: `合账` / `Hezhang`, generated words like `Ledgo`, others. Decide before M5 packaging.

---

## 15. History and rejected alternatives

This section accrues entries as design evolves. Each entry records what was considered, what was chosen, and why.

### 2026-04 — Initial design
- **Core as a single crate vs many.** Chose single `unbill-core`. Splitting premature until module boundaries prove stable and there is concrete reason to split.
- **CRDT library.** Chose Automerge over Yjs. Rationale: native Rust (Yjs is JS-first with bindings), JSON-like document model fits a ledger naturally, autosurgeon provides excellent ergonomics for Rust structs.
- **Transport.** Chose Iroh over libp2p. Rationale: simpler API, built-in NAT traversal with relay fallback, free public relays, NodeId-as-identity.
- **Persistence.** Chose SQLite over flat files or an embedded KV. Rationale: transactions matter for compaction, ubiquitous on all target platforms, well-understood tooling.
- **No central services.** Confirmed: no auth server, no backup server, no analytics. This is a hard constraint, not a budget constraint.

---

## Appendix A: Glossary

- **CRDT** — Conflict-free Replicated Data Type. A data structure that can be replicated across devices, updated independently, and merged without conflict resolution logic.
- **Op / operation** — A single unit of change in a CRDT. Append-only; never modified.
- **Head** — The latest operation(s) in a CRDT document. A document's state is uniquely identified by its set of heads.
- **Tombstone** — A marker indicating logical deletion. The tombstoned item remains in storage so that references don't break and so concurrent concurrent edits can still be resolved.
- **Amendment** — In unbill, a record that modifies a pre-existing Bill. Append-only; edits to bills are always amendments, never in-place mutations.
- **NodeId** — A device's identity in Iroh. An Ed25519 public key.
- **ALPN** — Application-Layer Protocol Negotiation. A TLS/QUIC field we use to label our sync protocol version.

## Appendix B: Dependencies (tentative)

- `automerge` — CRDT core.
- `autosurgeon` — struct ↔ Automerge mapping.
- `iroh` — P2P networking.
- `tokio` — async runtime.
- `rusqlite` or `sqlx` — SQLite access.
- `serde`, `serde_json`, `ciborium` — serialization.
- `ulid` — ID generation.
- `anyhow`, `thiserror` — error handling.
- `tracing`, `tracing-subscriber` — logging.
- `clap` — CLI argument parsing (cli crate only).
- `tauri`, `tauri-build` — desktop app (tauri crate only).
- `async-trait` — async in traits.
- `dashmap` — concurrent map.
- `dirs` — platform data directories.
