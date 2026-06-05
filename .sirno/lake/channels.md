---
core.name: Channels
core.desc: The review neighborhood for Unbill communication boundaries.
core.category:
  - core.concept
core.belongs:
  - unbill
---

Channels are the communication boundaries between Unbill roles.

The symmetric channel connects devices to devices.
It is a peer-equal data plane for CRDT convergence and device join.

The asymmetric channel connects devices to consoles.
It carries control operations,
Automerge sync rounds,
and subscription events from the device toward the console.

The two channel families stay separate so each boundary is exactly as complex as its job requires.
