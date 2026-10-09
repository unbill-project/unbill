---
core.desc: The separation between accounting roles and authorized sync peers.
core.name: Users And Devices
meta:
  frozen:
    - reviewed
core.category:
  - core.concept
core.belongs:
  - unbill
core.refines:
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

`add_device` returns an error when the `NodeId` is already authorized.
Callers that want idempotent behavior catch the `DuplicateDevice` check error.

A device name belongs to its ledger device record, not to local device metadata.
`DeviceLabel` is a required, nonempty, validated name. `NewDevice` and the join
request require it; the joining device supplies its own name. The host records
that name with the TLS-authenticated joining NodeId before returning the snapshot.
Renaming requires both a ledger ID and a NodeId, changes the CRDT, and propagates
through ledger sync and ledger-update events. There is no personal device alias API.
The ledger creator initially uses the shared name "Unnamed device".
Ledger-scoped device views use the selected ledger's label. Across-ledger peer
views choose the label from the lowest ledger ID, deterministically.
Old schemas and unlabeled documents are not supported by this change.
