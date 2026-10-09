---
core.name: Data Model
core.desc: The ledger, user, device, bill, and token objects that make up Unbill state.
core.category:
  - core.concept
core.belongs:
  - unbill
core.refines:
  - design-principles
---

A ledger is an independent shared workspace with a fixed currency.
It is backed by an Automerge document and carries the durable group record.

A user is a named person or role inside one ledger.
User membership is append-only; users are not login identities.
Each user has a shared ledger-scoped archived flag, initially false.
Archiving changes presentation and new-bill defaults without removing history or balances.

A device is an authorized sync peer identified by a `NodeId`.
Device membership is append-only, ledger-scoped, and separate from users.
Each device record has a required `DeviceLabel`, its shared name in that ledger.
Names may be changed without changing device identity or authorization.

A bill is an expense record with payer shares, payee shares, amount in cents,
timestamp, description, and optional `prev` links to superseded bills.
Effective bills are the bills not named by another bill's `prev`.

Invitation tokens are short-lived join credentials.
They authorize device join flows but are not part of shared ledger state.

Domain types use typed IDs and opaque wrappers.
`Ulid` names ledgers, bills, and users.
`Timestamp` is Unix milliseconds.
`Currency` is an ISO 4217 alphabetic code.
`NodeId` is the device identity string owned by the network boundary.
`SecretKey` is raw Ed25519 key material and remains opaque to the model crate.

The shared ledger stores durable collaborative state only:
ledger metadata, users, bills and supersession links, and authorized device IDs and labels.
Device-local storage holds the device key,
ledger metadata caches,
and pending invitation tokens.
Each kind of record has typed storage operations.
Runtime UI state and projection caches are not replicated ledger facts.
