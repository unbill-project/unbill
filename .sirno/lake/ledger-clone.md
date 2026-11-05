---
core.desc: Independent copies of ledger content without inherited peer access.
core.name: Ledger Clone
core.category:
  - core.concept
core.belongs:
  - console-service
  - ui-shared-model
---

Cloning creates a new active ledger on the connected device with a fresh ID,
creation timestamps, and Automerge history. Its name is the source name plus
` (copy)`, shortened at a UTF-8 boundary if needed to fit the ledger name limit.
Cloning an archived ledger is allowed and does not restore or modify the source.

The copy keeps the currency, users (including archive flags), and all bills
with their IDs, timestamps, share weights, and amendment links. Keeping bill
IDs preserves split rounding and keeping amendments preserves conflicts.
The connected device is the only authorized device in the copy; copied bill
entries are attributed to it to satisfy the ledger's author membership invariant.
Source peer permissions and invitations do not carry over.

The console persists the new document using the normal ledger creation
metadata and sync path. Graphical Ledger Settings places **Clone Ledger**
immediately above Archive Ledger or Restore Ledger. Successful cloning
refreshes the ledger list without changing the open source ledger. The terminal
settings popup offers a clone shortcut and the CLI offers `ledger clone`.
