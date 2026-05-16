---
name: Unbill Daemon
desc: The long-running local device process used by local consoles.
category:
  - concept
belongs:
  - applications
refines:
  - deployment-topologies
---

`unbill-daemon` is the background process that owns local device state.
It exposes that state to other processes on the same machine.

The daemon holds the exclusive `FsStore` file lock for the data directory,
preventing concurrent writes from corrupting data.
It runs the Iroh endpoint accept loop for peer sync and join requests.
It serves the local RPC socket so CLI, TUI, and other local clients can issue commands
without touching storage directly.

The daemon prints `listening on: <node_id>` to stdout once the Iroh endpoint is bound
and the RPC socket accepts connections.
That line is the readiness signal for automated tooling.
All other output goes to stderr.

The daemon runs until killed or until a fatal network or storage error occurs.

Implementation is contained in `main.rs`.
It opens `FsStore`,
opens `LocalAsymChannel` over the store,
and runs channel accept loop and RPC serving concurrently.
The process exits cleanly when either task returns.

Tracing goes to stderr so stdout remains reserved for the readiness line.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [applications](applications.md)
- belongs (from): (none)

> **Sirno generated links end.**
