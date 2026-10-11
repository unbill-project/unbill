---
core.name: Asymmetric Channel
core.desc: The device-to-console trait and local, RPC, and HTTP implementations.
core.category:
  - core.concept
core.belongs:
  - channels
  - workspace-layout
core.refines:
  - device-console-split
---

`unbill-asymmetric-channel` defines the boundary between devices and consoles.
The console depends on the trait,
not on any concrete device process or transport.

The trait has four planes plus identity.
The control plane creates invitations, joins ledgers, and triggers peer sync.
The data plane runs one Automerge sync round per `asym_sync` call.
The metadata plane lists ledgers and reads and writes ledger metadata.
Device names travel only in ledger CRDT data; joining requires a typed name
for the joining device.
It does not expose arbitrary metadata, secret keys, or pending invitation storage.
The subscription plane returns device-originated `AsymChannelEvent` values.

`AsymChannelEvent` currently carries `LedgerUpdated { ledger_id }`.
When a console receives it,
the console polls the channel and refreshes its in-process projection.

Implementations:
`LocalAsymChannel` wraps `UnbillDevice` in-process and forwards calls directly.
It translates `ServiceEvent` into `AsymChannelEvent` through a background task.

`RpcAsymChannel` uses tarpc over a Unix local socket.
The server side lives in `rpc::serve`.
Peer-sync requests set a 120-second RPC deadline for connection, exchange, and
persistence. Deadline expiry reports Peer sync timed out after 120 seconds.
Other RPC operations retain their existing request deadlines.
Event delivery is polled through per-connection queues.
Poisoned queue locks reuse the contained event vector;
appending or draining events requires no multi-step state repair.

`HttpAsymChannel` wraps a `reqwest::Client` with base URL and bearer token.
It maps trait methods to the `unbill-server` REST API.
It also runs an SSE task that connects to the events endpoint,
parses `data:` lines as JSON events,
ignores other SSE fields,
and reconnects after drops.

The identity surface also exposes service build information; RPC and HTTP return the connected service build, while Local returns the in-process build.

AsymChannel exposes typed set_ledger_archived and transports the local archived flag in ledger summaries. Local and RPC channels change the device preference; HTTP changes the server device preference. Archive changes use the existing ledger update notifications and never enter replicated ledger documents.
