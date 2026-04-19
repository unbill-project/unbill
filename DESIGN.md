# unbill

A fully decentralized, peer-to-peer, offline-first bill-splitting app. No central server. No accounts. Data lives on users' devices.

## What it is

Groups share a **ledger** — a history of expenses: who paid, how much, and how costs are split. Each participant runs unbill on their own device. Devices sync directly when online; changes made offline propagate automatically on reconnect.

## What it is not

- A payment processor. unbill records obligations; settlement happens outside the app.
- A general-purpose accounting tool.
- A commercial service. No SaaS, no data monetization, no lock-in.
- Hardened against malicious group members in v1. Trust model is "friends and family."

## Target user

A person splitting expenses with a small group — roommates, a couple, a travel party — who wants data ownership, no account, no subscription, multi-device sync without cloud, and offline-first operation.

## Design principles

1. **Offline-first.** Every operation works without network. Sync is opportunistic.
2. **Data lives with users.** No server required. If the author disappears, existing installs keep working forever.
3. **CRDTs over consensus.** State is a deterministic function of observed operations. We never ask which device has "the truth."
4. **Append-only at the data layer.** Members, devices, and bills are never removed. Amending a bill creates a new bill with a fresh ID and a `prev` list naming the bill(s) it supersedes; superseded bills are excluded from the effective view. The ledger is an event log; the UI renders a projection.
5. **One layer per concern.** Persistence, networking, business logic, and UI are separate and do not leak into each other.
6. **Abstract only where a real alternative exists.** The storage backend is a trait; the CRDT engine is not.
7. **Conservative about CRDT content.** Device preferences, UI state, and caches stay out of the synced document.
8. **Rust engine, any UI.** The core library defines what unbill is. Frontends are thin consumers.

## Data model

### Ledger
A shared expense context — "our household," "the Iceland trip." Contains members, authorized devices, and bills. Each ledger is independent; a user may have many.

### Member
A named participant in a ledger, identified by a stable user ID. Members have no device binding — any authorized device may record bills on behalf of any member. Members are append-only; once added they are never removed. A user must be a member before they can appear as a payer or participant on any bill.

### Device
A physical device authorized to sync a ledger, identified by its Ed25519 `NodeId`. Devices are associated with the ledger, not with individual members. Any device in a ledger's device list may submit bills for any member — the trust model is "everyone in the group trusts everyone else's device." Devices are append-only; once authorized they are never removed from the list.

### Bill
An expense entry: who paid, how much, and how the cost is split. Every bill has a unique ID. Bills carry a `prev` list of IDs of the bills they supersede — empty for original bills. A bill is **effective** if no other bill's `prev` references its ID. Amending a bill means creating a new bill whose `prev` points to the bill(s) being replaced. `prev` may reference multiple bills, enabling merges. Bills are never deleted.

### Split model
All bills use relative share weights. Equal split is everyone gets weight 1. Different weights express proportional or exact-amount splits. One model covers all cases — no separate split modes.

### Invitation
A short-lived in-memory token allowing a new member to join. Never persisted or synced; consumed on first use or expiry.

## Security and privacy

**Defended:** Passive eavesdroppers (QUIC+TLS 1.3 via Iroh). Device impersonation (Ed25519 key verification at handshake). Cross-group leakage (only devices in the ledger's device list are accepted).

**Not defended in v1:** Malicious insiders, device compromise, device revocation (devices cannot be removed; a compromised device retains access until the ledger is abandoned), relay metadata exposure.

**Telemetry:** None. Outbound connections are limited to Iroh peer discovery, Iroh relay fallback, and direct peer sync. No analytics, no error reporting, no update checks by default.

## Roadmap

- **M0** Workspace skeleton, build passes, design docs in place.
- **M1** Core data model and LedgerDoc, in-memory only.
- **M2** Flat-file persistence, UnbillService startup, CLI basics.
- **M3** P2P sync via Iroh, device invite/join flow.
- **M5** Desktop GUI (Tauri + React).
- **M6+** Amendment conflict UX, mobile, multi-currency, formal verification.

## Open questions

1. Mobile notification strategy (iOS backgrounding constraints).
2. Backup and restore for the "phone lost" scenario.
3. App name — "unbill" is a placeholder.

## Glossary

- **CRDT** — Conflict-free Replicated Data Type. A data structure that can be updated independently on any device and merged without conflict resolution logic.

- **Amendment** — A new bill with a fresh ID whose `prev` list names the bill(s) it supersedes. A superseded bill is excluded from the effective view. `prev` may name multiple bills, allowing several bills to be merged into one.
- **NodeId** — A device's identity: an Ed25519 public key derived from a per-device secret.
- **Op / operation** — A single unit of change in a CRDT. Append-only; never deleted.
- **Head** — The latest operation(s) in a CRDT document. A document's state is uniquely identified by its set of heads.
- **ALPN** — Application-Layer Protocol Negotiation. Used in TLS/QUIC to identify the sync protocol version.
