---
core.name: UniFFI Console
core.desc: The shared Console API and platform-specific construction surface for foreign-language clients.
core.category:
  - core.concept
core.belongs:
  - workspace-layout
---

`unbill-ffi` exposes `Console` to Python and Swift through UniFFI.
Public console naming belongs to this shared interface rather than language-specific aliases.
The underlying orchestration remains in `UnbillConsole`.

Construction explicitly selects the deployment topology.
`open_in_process` owns a SQLite-backed device service and its background tasks
in the caller's process. Desktop storage retains exclusive directory ownership.
`open_rpc` connects to an existing daemon using the configured data directory
and optional socket override, without opening storage itself.
Both constructors return the same console operation surface.

In-process construction is available on all supported native platforms.
Daemon construction and RPC dependencies are absent on Android and on iOS device
and Simulator builds. Mac Catalyst remains a desktop target.
Unsupported constructors are omitted at compilation rather than represented by
runtime-error stubs.

Generated bindings and headers belong to the target library that supplied their
UniFFI metadata. Apple builds select matching Swift source and headers for each
XCFramework variant. Compile checks require in-process construction on iOS and
reject daemon construction there; desktop variants accept both constructors.

observe_ble exposes the Console BLE watch subscription through a typed FfiBleObserver callback. It delivers the initial snapshot and changed snapshots on the Console runtime. The returned FfiSubscription cancels delivery explicitly or on drop. BLE DTOs preserve state, phases, transport path and optional verified node identity.
