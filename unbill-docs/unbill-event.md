---
core.desc: The event enum used for runtime notifications across services.
core.name: Unbill Event
core.category:
  - core.concept
core.belongs:
  - workspace-layout
core.refines:
  - sync-behavior
---

`unbill-event` contains `ServiceEvent`.
It has no dependencies and no logic.

`LedgerUpdated { ledger_id }` means a ledger was saved.
The store emits it after every successful `save_ledger`,
whether the write path was local mutation, remote sync, or join.

`PeerConnected { ledger_id, peer }` means a peer opened a sync connection.
`PeerDisconnected { ledger_id, peer }` means that connection closed.
`SyncError { ledger_id, peer, error }` means a sync session failed.

Events are informational.
No action is required on receipt.

Events are distributed through Tokio broadcast channels created by stores and endpoints,
then consumed by devices and consoles.

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [workspace-layout](workspace-layout.md)
- core.belongs (from): (none)

> **Sirno generated links end.**
