# unbill-core

The library that defines what unbill is. Owns the CRDT document model, persistence, P2P networking, sync protocol, and all business logic (bill splitting, settlement). Every frontend is a thin consumer of this crate.

This crate contains no CLI argument parsing, Tauri command wiring, or UI state.

## Public API

The primary entry point is `UnbillService`. Frontends create one instance at startup and call its async methods.

**Ledger lifecycle:** create a named ledger with a fixed currency; list all ledgers; delete; export to bytes; import from bytes.

**Bills:** add a bill (payer, amount, description, share weights); amend an existing bill; tombstone-delete; restore; list as effective (projected) bills.

**Members:** add a member directly by user ID and display name; remove a member (tombstone); list current (non-removed) members. Members are named participants only — they carry no device binding. The full invite/join flow (out-of-band token, join URL) is deferred to M4.

**Devices:** devices are associated with the ledger, not with individual members. Any device in a ledger's device list may record bills on behalf of any member. Device management (add/remove) is exposed in M4 alongside the invite/join flow.

**Settlement:** given a user ID, compute the minimum set of transactions that clears all of that user's debts and credits across every ledger they participate in.

**Events:** subscribe to a broadcast channel receiving ledger updates, peer connection changes, and sync errors.

Key model types: `Ulid`, `Timestamp`, `Currency`, `NodeId`, `InviteToken`, `LedgerMeta`, `Member`, `EffectiveBill`, `Settlement`.

## Invariants

- All entity IDs (`ledger_id`, `bill.id`, `user_id`, `amendment.id`) are `Ulid` — globally unique, monotonically ordered, never reused.
- Bills are append-only. Logical deletion is tombstoning; removing a bill from the underlying vector is forbidden.
- Amendments are append-only. Editing a bill means adding an `Amendment` record, never mutating existing fields.
- `amount_cents` is non-negative. Refunds are modeled as separate bills with reversed payer/participant roles.
- A ledger's currency is a valid ISO 4217 code and is fixed at creation.
- Device node IDs and bill creator fields are valid Ed25519 public keys.
- Member IDs are stable. A member is identified solely by their `user_id`; no device is bound to a specific member.
- `InviteToken` is 32 bytes from `OsRng`, hex-encoded. Never written to disk.
- The payer and every share participant in a bill must be active (non-removed) members of the ledger at the time the bill is added. Attempting to add a bill referencing a non-member returns `UserNotMember`.

## Failure modes

| Error | Meaning |
|-------|---------|
| `LedgerNotFound` | Querying a ledger ID that does not exist |
| `BillNotFound` | Amending, deleting, or restoring a bill ID that does not exist |
| `MemberNotFound` | Removing a user ID that is not an active member |
| `UserNotMember` | Adding a bill whose payer or participant is not an active member |
| `InvalidInvitation` | Join token is expired, already used, or unrecognized |
| `NotAuthorized` | Peer attempted to sync a ledger they are not a member of |
| `Storage(Io)` | File I/O failure in `FsStore` |
| `Storage(Serialization)` | Corrupt or unreadable persisted data |

Callers (CLI, Tauri) are responsible for mapping these to user-facing messages.
