---
name: Users And Devices
desc: The separation between accounting roles and authorized sync peers.
category:
  - concept
belongs:
  - unbill
refines:
  - data-model
---

Users and devices are separate because people and hardware do not map one-to-one.
A person may use many devices,
and a shared device may be used by more than one person.

Authorization happens at the device level.
Bill semantics reference users.

A user is a ledger-internal accounting dimension.
It is independent of device, login identity, or operating system account.
Any member can operate any user.
Multiple users may represent the same real person,
and one user may represent several real people when that fits the group.

A device enters a ledger in exactly two ways:
the creating device is added automatically when the ledger is created,
or a host adds a subsequent device through the join protocol.
There is no device removal in the current design.

`add_device` is idempotent.
Adding an already authorized `NodeId` is a no-op.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from): (none)

> **Sirno generated links end.**
