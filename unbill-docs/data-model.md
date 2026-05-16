---
name: Data Model
desc: The ledger, user, device, bill, and token objects that make up Unbill state.
category:
  - concept
belongs:
  - unbill
refines:
  - design-principles
---

A ledger is an independent shared workspace with a fixed currency.
It is backed by an Automerge document and carries the durable group record.

A user is a named person or role inside one ledger.
Users are append-only shared records and are not login identities.

A device is an authorized sync peer identified by a `NodeId`.
Devices are append-only, ledger-scoped, and separate from users.

A bill is an expense record with payer shares, payee shares, amount,
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
ledger metadata, users, bills and supersession links, and authorized device IDs.
Local metadata stores saved users, device labels, pending tokens, UI state, and caches.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from): (none)

> **Sirno generated links end.**
