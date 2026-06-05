---
core.desc: What Unbill deliberately does not own.
core.name: Project Boundaries
core.category:
  - core.concept
core.belongs:
  - unbill
core.refines:
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
and the current service aggregates known users from local ledger state rather than from a
separate saved-user table.

Unbill has no telemetry, analytics, hosted account system, or server-backed authority model.
Remote access can be hosted by a user,
but no service becomes the permanent owner of a ledger once peers have copies.

The current security target does not include malicious insiders,
compromised devices, revocation, or relay metadata protection.
Groups that need those guarantees are outside the current design target.

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [unbill](unbill.md)
- core.belongs (from): (none)

> **Sirno generated links end.**
