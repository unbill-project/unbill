---
name: Sync Behavior
desc: The user-initiated CRDT sync and event propagation model.
category:
  - concept
belongs:
  - unbill
  - channels
refines:
  - design-principles
---

Sync is user-initiated or daemon-driven.
A user or integration explicitly requests a sync round,
or the daemon triggers one on a periodic interval.

The daemon optionally runs a background sync loop that periodically
dials all known peers (other devices that share at least one ledger)
and runs the symmetric sync protocol with each.
The loop is enabled by setting `UNBILL_SYNC_INTERVAL_SECS` to the
desired interval in seconds; without this variable the daemon does
not poll.
Failures for individual peers are logged and do not stop the loop.

Device-to-device sync happens through the symmetric channel.
Peers exchange ledger lists,
authorize accepted ledgers by device membership,
run Automerge sync messages for those ledgers,
save touched documents,
and disconnect.

Device-to-console sync happens through the asymmetric channel.
The console sends one Automerge sync message per request,
and the device responds with its own sync message or no message when it has nothing new.

After remote changes are applied and saved,
the store emits `LedgerUpdated`.
The device forwards that fact through the asymmetric channel.
The console responds by pulling a fresh sync round.

UI layers receive materialized service events and projected views.
CRDT details are absorbed by the console library before reaching frontend components.

```mermaid
flowchart LR
    Device["Device"]
    Asym["Asymmetric channel"]
    Console["Console"]
    Host["Host-specific UI bridge"]
    UI["UI"]

    Device -->|"LedgerUpdated"| Asym
    Asym --> Console
    Console -->|"projected ServiceEvent and views"| Host
    Host --> UI
```

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [channels](channels.md)
  - [unbill](unbill.md)
- belongs (from): (none)

> **Sirno generated links end.**
