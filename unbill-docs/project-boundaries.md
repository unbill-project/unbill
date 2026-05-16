---
name: Project Boundaries
desc: What Unbill deliberately does not own.
category:
  - concept
belongs:
  - unbill
refines:
  - design-principles
---

Unbill records obligations but does not move money.
It has no payment rail, no bank integration layer, and no general accounting scope.

Synced state excludes UI state, caches, device labels, pending invitation tokens,
and other machine-local metadata.
Those facts stay local so peers do not have to converge on personal preferences
or device-specific convenience data.

Devices are authorized per ledger.
They are not bound to specific users,
and saved users on one device are separate from ledger users in shared state.

Unbill has no telemetry, analytics, hosted account system, or server-backed authority model.
Remote access can be hosted by a user,
but no service becomes the permanent owner of a ledger once peers have copies.

The current security target does not include malicious insiders,
compromised devices, revocation, or relay metadata protection.
Groups that need those guarantees are outside the current design target.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from): (none)

> **Sirno generated links end.**
