---
name: Unbill Model
desc: The domain type and document crate shared across Unbill.
category:
  - concept
belongs:
  - workspace-layout
refines:
  - data-model
---

`unbill-model` contains shared domain types and the Automerge document layer.

The crate defines typed identifiers, timestamps, currencies, device identities,
secret keys, invite tokens, bills, shares, effective bill wrappers,
ledger records, users, devices, invitations,
input structs (`NewBill`, `NewUser`, `NewUserName`, `NewDevice`, `NewLedger`),
`LedgerMeta` for lightweight ledger summaries without Automerge bytes,
and error enums.

`LedgerDoc` wraps an Automerge document and provides typed read and write operations
such as `add_bill`, `add_user`, `add_device`, `merge`, and sync helpers.

Types that appear in Automerge documents implement `autosurgeon::Reconcile` and `Hydrate`.
`InviteToken` is memory-only and is never synced across devices.
`Invitation` wrapping it may be serialized for device-local storage.

`EffectiveBills` is a thin wrapper over the vector of bills left after supersession filtering.

`UnbillError` covers domain-level failures (missing ledgers, users, bills, devices),
validation, authorization, Automerge and reconciliation errors,
network and URL errors, ID parsing, and configuration errors.
`StorageError` covers store and transport failures so storage concerns do not bleed into domain code.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [workspace-layout](workspace-layout.md)
- belongs (from): (none)

> **Sirno generated links end.**
