---
name: Store Server
desc: MPSC actor that serializes all LedgerStore access with atomic compound operations.
belongs:
  - unbill-storage
---

\`StoreServer\` serializes all store access through an MPSC channel.
A single background tokio task owns the raw \`dyn LedgerStore\`
and processes commands sequentially.
\`StoreServer\` does NOT implement \`LedgerStore\` —
no component other than the internal consumer ever holds a raw store reference.
The type system enforces this: functions that need store access
take \`&StoreServer\`, not \`&dyn LedgerStore\`.

Individual operations (list, load, save) and compound operations
(asym_sync, create_invitation, consume_invitation, add_device_to_ledger,
merge_and_save_ledger, persist_joined_ledger, collect_peers)
are all public methods on \`StoreServer\`.
Compound operations execute as single MPSC commands,
guaranteeing atomicity for read-modify-write sequences.

\`merge_and_save_ledger\` handles the sync session save phase:
it atomically loads the current stored version,
merges the synced document via \`LedgerDoc::merge\`,
and saves the combined result.
This prevents concurrent operations from overwriting each other.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill-storage](unbill-storage.md)
- belongs (from): (none)

> **Sirno generated links end.**
