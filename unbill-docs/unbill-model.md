---
name: Unbill Model
desc: The logic-free domain type crate shared across Unbill.
category:
  - concept
belongs:
  - workspace-layout
refines:
  - data-model
---

`unbill-model` contains shared domain data types.
It owns no business logic and no I/O.

The crate defines typed identifiers, timestamps, currencies, device identities,
secret keys, invite tokens, bills, shares, effective bill wrappers,
ledger records, users, devices, invitations, and error enums.

Types that appear in Automerge documents implement `autosurgeon::Reconcile` and `Hydrate`.
`InviteToken` is memory-only.
It may serialize for in-process use,
but it is never written to a store or synced across devices.

`EffectiveBills` is a thin wrapper over the vector of bills left after supersession filtering.

`UnbillError` covers domain-level failures such as missing ledgers or users.
`StorageError` covers store and transport failures so storage concerns do not bleed into domain code.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [workspace-layout](workspace-layout.md)
- belongs (from): (none)

> **Sirno generated links end.**
