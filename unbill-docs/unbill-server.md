---
core.desc: The standalone HTTP backend for remote consoles operating one device namespace.
core.name: Unbill Server
core.category:
  - core.concept
core.belongs:
  - applications
core.refines:
  - deployment-topologies
---

`unbill-server` is a standalone HTTP server exposing the device API used by `HttpAsymChannel`.
It is the remote backend for one device identity and one set of ledgers.

The server lets a remote console reach the device API over a network instead of a local socket.
A single running instance serves authenticated clients identified by one static API key.

All endpoints require a bearer token.
Missing or wrong tokens receive unauthorized responses.

The API covers ledgers,
ledger metadata,
Automerge sync rounds,
invitation creation,
ledger join,
peer sync triggers,
device ID,
device metadata blobs,
and server-sent events.

The ledger sync endpoint accepts binary Automerge sync message bytes.
It returns response bytes when the server has a message,
or no content when it has nothing new.
Malformed sync bytes are bad requests.

The event endpoint opens an SSE stream.
The server subscribes to device events and forwards ledger updates as JSON data fields.
The stream has no terminal event.
Clients reconnect when it drops.

Device metadata key names may contain only ASCII letters,
digits,
hyphens,
underscores,
and dots.
Invalid keys are rejected before touching the store.

Configuration comes from environment variables.
`API_KEY` is required.
`PORT` defaults to 8080.
The data directory follows `UNBILL_DATA_DIR` when set,
otherwise the platform default from `unbill-store-fs`.

Boundaries:
one API key,
one device namespace,
no multi-tenancy,
no ledger-level access control beyond the API key,
and no built-in TLS termination.
TLS belongs at a reverse proxy.

Implementation:
`main.rs` reads configuration,
opens `FsStore` and `UnbillDevice`,
spawns the Iroh accept loop,
and starts Axum.
`router.rs` owns routes,
handlers,
auth middleware,
device key validation,
and SSE streaming.
`lib.rs` exports router construction so tests can call it without TCP.

Errors map malformed sync messages to bad request.
Other service errors become internal server errors with message bodies.
Only `LedgerUpdated` events are forwarded over SSE;
other events and lagged receiver errors are skipped.

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [applications](applications.md)
- core.belongs (from):
  - [server-http-api](server-http-api.md)

> **Sirno generated links end.**
