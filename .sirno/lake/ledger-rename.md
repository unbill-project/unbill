---
core.desc: A shared ledger name that converges through document synchronization.
core.name: Ledger Rename
core.category:
  - core.concept
core.belongs:
  - console-service
  - ui-shared-model
  - storage
---

A ledger name is a mutable scalar in the shared Automerge document. Renaming
trims surrounding whitespace and accepts a nonempty name of up to 100 UTF-8
bytes. The ledger ID, currency, creation time, members, bills, invitations,
and each device's archive preference are unchanged.

Rename uses ordinary document sync on every transport. Concurrent renames
converge using Automerge's deterministic scalar conflict resolution, not wall
clock time. Stores project the name from the saved document into list metadata;
a stale metadata write does not rename an existing document.
Older device builds that treat the name as immutable cannot accept a rename;
upgrade shared devices before renaming.

Ledger Settings offers Rename Ledger with the current name prefilled. Terminal
settings expose a rename shortcut and the CLI accepts `ledger rename <id> <name>`.
A rename refreshes the open ledger and the ledger list without changing selection.
