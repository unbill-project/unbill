---
name: Unbill Device
desc: The device-side service that owns storage, identity, sync, and channel server behavior.
category:
  - concept
belongs:
  - workspace-layout
  - channels
refines:
  - device-console-split
---

`unbill-device` implements the device role.
It owns a `StoreServer` (from `unbill-storage`),
derives the device `NodeId` from the store secret key,
runs the symmetric channel endpoint,
and serves the server side of asymmetric channel behavior.

`UnbillDevice` is a plain Rust struct.
It is not aware of the `AsymChannel` trait.
`LocalAsymChannel` wraps it,
while RPC and HTTP server code call the same methods directly.

One `UnbillDevice` instance exists per device process.
It holds the store and endpoint exclusively.
The symmetric endpoint is created lazily on first use when needed.
`accept_loop` binds a persistent endpoint and runs until it closes.

Pending invitation tokens are persisted in local device metadata.
The join URL format names the ledger ID, host node ID, and token.

`asym_sync` decodes one client Automerge sync message,
applies it to the loaded ledger document,
saves only when the document heads advance,
and returns the device's response message.
No-op sync rounds do not write and do not emit `LedgerUpdated`.

`UnbillDevice::open` calls init methods on the raw store
before wrapping it in a `StoreServer` (see `unbill-storage` docs).
All subsequent runtime access goes through the serialized channel.

`trigger_peer_sync` and `join_ledger` reuse the running endpoint when active.
Otherwise they bind an ephemeral endpoint for the operation and close it afterward.

Integration tests use `unbill-store-memory` to exercise the device surface
without touching disk or real network endpoints.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [channels](channels.md)
  - [workspace-layout](workspace-layout.md)
- belongs (from): (none)

> **Sirno generated links end.**
