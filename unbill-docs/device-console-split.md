---
name: Device Console Split
desc: The role separation between durable device replicas and transient user-facing consoles.
category:
  - concept
belongs:
  - unbill
refines:
  - design-principles
---

Unbill separates devices from consoles.

A device owns storage, the local device identity,
authorized ledger replicas,
peer-to-peer sync,
join handling,
and the event stream emitted when ledgers change.

A console owns display and user interaction.
It asks a device to create ledgers, add users, save bills,
create invitations, join ledgers, trigger peer sync,
and run Automerge sync rounds.

The split lets one machine run multiple consoles against the same device.
A daemon can keep syncing after a UI closes.
A remote browser console can operate a device hosted on a home server or VPS.
A mobile app can colocate both roles in one process when that fits the platform.

The same split also keeps business rules out of shell code.
Consoles render projected data and send complete commands.
They do not own durable state, settlement, conflict detection, or persistence layout.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from): (none)

> **Sirno generated links end.**
