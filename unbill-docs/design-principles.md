---
name: Design Principles
desc: The architectural commitments that keep Unbill local-first and understandable.
category:
  - concept
belongs:
  - unbill
---

Unbill is offline first.
Local work must not depend on network availability.

Unbill separates device and console roles.
The device owns storage and sync.
The console owns display and user interaction.
They communicate through the asymmetric channel.

Unbill uses CRDTs over consensus.
State is derived from observed operations rather than from one authoritative device.

Shared state is append-only where business semantics need history.
Users, devices, and bills are added rather than edited in place.
Bill corrections are new bills that supersede prior bills.

Projection is deterministic.
The UI renders effective bills, conflicts, and settlement from shared history,
not from mutable presentation tables.

The trust model is narrow.
Joined members are treated as equally trusted,
and the main threat is accidental or stranger access,
not malicious insiders.

Users are roles inside a ledger, not login identities.
Devices authorize sync, while bills reference users as accounting dimensions.

The ledger records state over provenance.
It captures what the group currently owes,
while social context carries who initiated a change.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from): (none)

> **Sirno generated links end.**
