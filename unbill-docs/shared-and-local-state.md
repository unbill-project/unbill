---
core.desc: The boundary between replicated ledger facts and one-device convenience data.
core.name: Shared And Local State
core.category:
  - core.concept
core.belongs:
  - unbill
core.refines:
  - data-model
---

Shared state is the minimum durable record peers need to agree on a ledger.
It includes ledger metadata, ledger users, bills, supersession links,
and authorized device `NodeId` values.

Local state makes one device usable.
It includes the device secret key,
ledger metadata caches,
device labels,
pending invitation tokens,
runtime UI state,
and other machine-specific metadata.

Shared and local state are stored separately.
Peers converge only on shared ledger data.
They do not converge on personal labels, saved user conveniences, clipboard contents,
or transient UI state.

Device labels and pending invitations are local metadata.
Invitation URLs and copied invitation text are local client concerns.
The current service lists all known users by aggregating ledger users across local ledgers,
not by maintaining a separate saved-user table.

```mermaid
flowchart TB
    subgraph Shared["Shared ledger state"]
        Ledger["Ledger metadata"]
        Users["Ledger users"]
        Bills["Bills and supersession links"]
        Devices["Authorized device NodeIds"]
    end

    subgraph Local["Device-local state"]
        Key["Device key"]
        MetaCache["Ledger metadata cache"]
        Labels["Device labels"]
        Pending["Pending invite tokens"]
        Ui["Runtime UI state"]
    end
```

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [unbill](unbill.md)
- core.belongs (from): (none)

> **Sirno generated links end.**
