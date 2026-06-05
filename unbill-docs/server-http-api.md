---
core.desc: The REST and SSE contract shared by unbill-server and HttpAsymChannel.
core.name: Server HTTP API
core.category:
  - core.concept
core.belongs:
  - unbill-server
  - asymmetric-channel
core.refines:
  - unbill-server
---

The HTTP API is authenticated with `Authorization: Bearer <api_key>`.
The server mounts the protected API under `/api/v1`.

```mermaid
flowchart TB
    Http["HttpAsymChannel"]
    Router["unbill-server /api/v1"]
    Device["UnbillDevice"]
    Store["LedgerStore"]
    Events["SSE events"]

    Http -->|"control, metadata, sync"| Router
    Router --> Device
    Device --> Store
    Device --> Events
    Events --> Http
```

Ledger endpoints list ledgers,
write ledger metadata,
exchange binary Automerge sync messages,
create invitations,
and join ledgers from invitation URLs.

Peer endpoints trigger one-shot sync to a known peer `NodeId`.

Device endpoints expose the current device ID
and read or write device metadata blobs under validated keys.

The events endpoint streams server-sent events.
`LedgerUpdated` events are serialized as JSON data fields.
Other event fields are ignored by the HTTP client,
and clients reconnect after connection errors or clean close.

The route surface is:
`GET /ledgers`,
`PUT /ledgers/{id}/meta`,
`POST /ledgers/{id}/sync`,
`POST /ledgers/{id}/invitations`,
`POST /ledgers/join`,
`POST /peers/{node_id}/sync`,
`GET /device/id`,
`GET /device/{key}`,
`PUT /device/{key}`,
and `GET /events`.

`POST /ledgers/{id}/sync` is the data-plane endpoint.
The client sends Automerge sync message bytes.
The server receives them into its document,
generates a response message when available,
saves when the document changed,
and returns either response bytes or no content.

The server rejects unparseable sync bodies as bad requests.
It maps unauthorized tokens to unauthorized responses.
Device metadata lookup returns `404` for missing keys,
which the client maps to `None`.
Other non-success statuses map to storage or channel errors on the client side.

`GET /ledgers` returns a JSON array of ledger metadata objects.
`PUT /ledgers/{id}/meta` accepts and returns ledger metadata as JSON.
`POST /ledgers/{id}/invitations` returns `201` with `{"url": "..."}`.
`POST /ledgers/join` accepts `{"url": "...", "label": "..."}` and returns `204`.
`GET /device/id` returns the node ID as `text/plain`.
SSE data fields carry `{"type": "LedgerUpdated", "ledger_id": "..."}`.

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [asymmetric-channel](asymmetric-channel.md)
  - [unbill-server](unbill-server.md)
- core.belongs (from): (none)

> **Sirno generated links end.**
