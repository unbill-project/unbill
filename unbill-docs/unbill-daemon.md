---
core.desc: The long-running local device process used by local consoles.
core.name: Unbill Daemon
core.category:
  - core.concept
core.belongs:
  - applications
core.refines:
  - deployment-topologies
---

`unbill-daemon` is the background process that owns local device state.
It exposes that state to other processes on the same machine.

The daemon holds the exclusive `FsStore` file lock for the data directory,
preventing concurrent writes from corrupting data.
It runs the Iroh endpoint accept loop for peer sync and join requests.
It serves the local RPC socket so CLI, TUI, and other local clients can issue commands
without touching storage directly.

The device layer prints `listening on: <node_id>` to stdout
inside `accept_loop` once the Iroh endpoint is ready.
That line is the readiness signal for automated tooling.
All other daemon output goes to stderr.

The daemon runs until killed or until a fatal network or storage error occurs.

Implementation is contained in `main.rs`.
It opens `FsStore`,
opens `LocalAsymChannel` over the store,
and runs channel accept loop, RPC serving,
and periodic peer sync concurrently in a `select!`.

When `UNBILL_SYNC_INTERVAL_SECS` is set to a positive value,
the daemon triggers peer sync on that interval.
When unset or zero the periodic sync arm is dormant.
The process exits cleanly when any task returns.

Tracing goes to stderr so stdout remains reserved for the readiness line.

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [applications](applications.md)
- core.belongs (from): (none)

> **Sirno generated links end.**
