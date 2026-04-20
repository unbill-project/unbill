# unbill-core

The library that defines what unbill is. Owns the CRDT document model, persistence, P2P networking, sync protocol, and all business logic (bill splitting, settlement). Every frontend is a thin consumer of this crate.

This crate contains no CLI argument parsing, Tauri command wiring, or UI state.

## Public API

The primary entry point is `UnbillService`. Frontends create one instance at startup and call its async methods.

**Ledger lifecycle:** create a named ledger with a fixed currency; list all ledgers; delete; export to bytes; import from bytes.

**Bills:** add a bill (payer, amount, description, share weights); amend an existing bill (appends a new entry with the same bill ID — the latest entry wins); list as effective (projected) bills. Bills are never deleted.

**Users:** add a user directly by user ID and display name; list all users. Users are named people in a ledger only — they carry no device binding and no creator metadata. Users are append-only and may not be removed. The full invite/join flow (out-of-band token, join URL) is deferred to M4.

**Devices:** devices are associated with the ledger, not with individual users. Any device in a ledger's device list may record bills on behalf of any user. A device is added to a ledger via the join flow. Devices are append-only and may not be removed. Human-readable device labels are device-local metadata and are not part of the shared device record.

**Settlement:** given a user ID, compute the minimum set of transactions that clears all of that user's debts and credits across every ledger they participate in.

**Events:** subscribe to a broadcast channel receiving ledger updates, peer connection changes, and sync errors.

Key model types: `Ulid`, `Timestamp`, `Currency`, `NodeId`, `InviteToken`, `LedgerMeta`, `User`, `EffectiveBill`, `Settlement`.

## Invariants

- All entity IDs (`ledger_id`, `bill.id`, `user_id`) are `Ulid` — globally unique, monotonically ordered, never reused.
- Bills are append-only and never deleted. Amending a bill means appending a new `Bill` entry with the same logical `id`; the entry with the latest `created_at` (ties broken by `created_by_device`) is the effective bill.
- `amount_cents` is non-negative. Refunds are modeled as separate bills with reversed payer/share roles.
- A ledger's currency is a valid ISO 4217 code and is fixed at creation.
- Device node IDs and bill creator fields are valid Ed25519 public keys.
- Device authorization is represented only by `NodeId` plus authorization timestamp in the shared ledger. Human-readable device labels are local metadata keyed by `NodeId`.
- User IDs are stable. A user is identified solely by their `user_id`; no device is bound to a specific user.
- User records store only the user's identity and display metadata (`user_id`, `display_name`, `added_at`).
- `InviteToken` is 32 bytes from `OsRng`, hex-encoded. Stored in `LedgerStore` and consumed on first use.
- No store-backed data is cached in memory. Every operation (ledger read, bill write, settlement, sync session) loads what it needs directly from `LedgerStore` and discards it when done. There is no in-memory shadow of store contents.
- The payer and every user referenced in a bill's share list must be users in the ledger at the time the bill is added. Attempting to add a bill referencing a non-user returns `UserNotInLedger`.

## Failure modes

| Error | Meaning |
|-------|---------|
| `LedgerNotFound` | Querying a ledger ID that does not exist |
| `BillNotFound` | Amending a bill ID that does not exist |

| `UserNotInLedger` | Adding a bill whose payer or share-list user is not an active user |
| `InvalidInvitation` | Join token is expired, already used, or unrecognized |
| `NotAuthorized` | Peer attempted to sync a ledger they are not a user of |
| `Storage(Io)` | File I/O failure in `FsStore` |
| `Storage(Serialization)` | Corrupt or unreadable persisted data |

Callers (CLI, Tauri) are responsible for mapping these to user-facing messages.
