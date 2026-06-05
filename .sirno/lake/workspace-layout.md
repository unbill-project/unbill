---
core.name: Workspace Layout
core.desc: The Rust workspace split across core crates, channel crates, UI crates, and applications.
core.category:
  - core.concept
core.belongs:
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
