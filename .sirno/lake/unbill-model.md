---
core.name: Unbill Model
core.desc: The domain type and document crate shared across Unbill.
core.category:
  - core.concept
core.belongs:
  - workspace-layout
core.refines:
  - data-model
---

`unbill-model` contains shared domain types and the Automerge document layer.

The crate defines typed identifiers, timestamps, currencies, device identities,
secret keys, typed device labels, invite tokens, bills, shares, effective bill wrappers,
ledger records, users, devices, invitations,
input structs (`NewBill`, `NewUser`, `NewUserName`, `NewDevice`, `NewLedger`),
`LedgerMeta` for lightweight ledger summaries without Automerge bytes,
and error enums.

`Currency::all` enumerates the complete currency catalog accepted by `Currency::from_code`,
using the same underlying ISO currency definitions.

`Timestamp::now` returns a system clock error when the clock precedes the Unix epoch.
Callers propagate that error through the domain or storage error boundary.

`LedgerDoc` wraps an Automerge document and provides typed read and write operations
such as `add_bill`, `add_user`, `set_user_archived`, `add_device`, `merge`, and sync helpers.

Types that appear in Automerge documents implement `autosurgeon::Reconcile` and `Hydrate`.
`InviteToken` is memory-only and is never synced across devices.
`Invitation` wrapping it may be serialized for device-local storage.
Token generation returns the system RNG error to its caller when secure randomness is unavailable.
Callers decide how to handle that failure.

`EffectiveBills` is a thin wrapper over the vector of bills left after supersession filtering.

`BleSnapshot` and its typed state, peer phase, and link-path records carry
process-local transport observations across console boundaries. They are not
Automerge ledger data; verified_node_id alone represents an authenticated
transport identity.

`UnbillError` covers domain-level failures (missing ledgers, users, bills, devices),
validation, authorization, Automerge and reconciliation errors,
network and URL errors, ID parsing, system clock errors, and configuration errors.
`StorageError` covers store and transport failures so storage concerns do not bleed into domain code.

Drafts may allocate a real `BillId` before insertion. `add_bill_with_id` preserves
that ID when saving, so deterministic rounding matches the preview. The verified
bill checks reject duplicate IDs; amendments use fresh IDs.

BleSnapshot keeps discovered_node_ids separate from verified peer identity. These are known ledger IDs correlated with peers in the current snapshot, not newly authenticated identities. observed_node_ids combines current matches and verified peer identities while the transport is Active, and returns empty when peers is empty; connected_node_ids requires Connected plus a verified identity. indicator_node_ids supplies the two lists for display, with connection taking precedence. Historical discovery alone never produces a badge. Missing discovery fields in older wire snapshots default to an empty list.
