---
name: Asymmetric Channel
desc: The device-to-console trait and local, RPC, and HTTP implementations.
category:
  - concept
belongs:
  - channels
  - workspace-layout
refines:
  - device-console-split
---

`unbill-asymmetric-channel` defines the boundary between devices and consoles.
The console depends on the trait,
not on any concrete device process or transport.

The trait has three planes.
The control plane creates invitations, joins ledgers, and triggers peer sync.
The data plane runs one Automerge sync round per `asym_sync` call.
The subscription plane returns device-originated `AsymChannelEvent` values.

`AsymChannelEvent` currently carries `LedgerUpdated { ledger_id }`.
When a console receives it,
the console polls the channel and refreshes its in-process projection.

Implementations:
`LocalAsymChannel` wraps `UnbillDevice` in-process and forwards calls directly.
It translates `ServiceEvent` into `AsymChannelEvent` through a background task.

`RpcAsymChannel` uses tarpc over a Unix local socket.
The server side lives in `rpc::serve`.
Event delivery is polled through per-connection queues.

`HttpAsymChannel` wraps a `reqwest::Client` with base URL and bearer token.
It maps trait methods to the `unbill-server` REST API.
It also runs an SSE task that connects to the events endpoint,
parses `data:` lines as JSON events,
ignores other SSE fields,
and reconnects after drops.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [channels](channels.md)
  - [workspace-layout](workspace-layout.md)
- belongs (from):
  - [server-http-api](server-http-api.md)

> **Sirno generated links end.**
