---
name: Workspace Layout
desc: The Rust workspace split across core crates, channel crates, UI crates, and applications.
category:
  - concept
belongs:
  - unbill
---

The repository is a Rust workspace.
The central rules live in reusable crates,
and applications stay thin around those crates.

Core crates:
`unbill-model` holds domain data types,
`unbill-storage` owns Automerge document and store traits,
`unbill-store-fs` and `unbill-store-memory` provide store backends,
and `unbill-event` carries runtime notifications.

Channel crates:
`unbill-symmetric-channel` owns device-to-device Iroh sync and join protocols.
`unbill-asymmetric-channel` owns the device-to-console trait and its local, RPC, and HTTP transports.

Role crates:
`unbill-device` is the device-side service.
`unbill-console` is the console-side orchestration library that shells use.

UI crates:
`unbill-tauri` exposes the desktop bridge and host.
`unbill-ui-components` provides shared Leptos components for frontend apps.

Applications:
`unbill-cli`, `unbill-tui`, `unbill-daemon`, `unbill-server`,
`unbill-ui-native`, and `unbill-ui-remote` adapt the same design to different runtimes.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from):
  - [asymmetric-channel](asymmetric-channel.md)
  - [fs-store](fs-store.md)
  - [memory-store](memory-store.md)
  - [symmetric-channel](symmetric-channel.md)
  - [ui-components](ui-components.md)
  - [unbill-console](unbill-console.md)
  - [unbill-device](unbill-device.md)
  - [unbill-event](unbill-event.md)
  - [unbill-model](unbill-model.md)
  - [unbill-storage](unbill-storage.md)

> **Sirno generated links end.**
