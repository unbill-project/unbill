---
core.desc: The native SwiftUI frontend for Apple platforms, built alongside the Tauri desktop shell.
core.name: Unbill Apple
core.category:
  - core.concept
core.belongs:
  - frontend-ui
  - applications
core.refines:
  - ui-shared-model
---

`unbill-apple` is the native SwiftUI frontend for Apple platforms.
It is built alongside the Tauri desktop shell and its Leptos frontend, not as a replacement.
It targets iPhone and iPad as one universal build and macOS through Mac Catalyst.

The app is backed by the real Rust core.
`ConsoleClient` is the single Swift boundary, an async protocol over the `UnbillConsole` orchestration surface.
`RustConsoleClient` is its only implementation: an actor that calls `unbill-ffi` and runs the synchronous bridge off the main thread.
There is no mock backend.

`unbill-ffi` is a Cargo workspace member that bridges `UnbillConsole` to Swift with UniFFI.
It mirrors the aggregated command granularity of the Tauri bridge rather than the raw console methods:
bootstrap, ledger detail, create ledger, create and import users, save bill, resolve conflict,
create invitation, join, sync, and a `ServiceEvent` callback stream.
The DTO assembly is ported from `unbill-tauri`; a shared `unbill-shell` crate would remove that duplication.

The project lives at `apps/unbill-apple`, outside the Cargo workspace.
An `xcodegen` `project.yml` defines one application target for iOS device, simulator, and Mac Catalyst.
`build-rust.sh` compiles `unbill-ffi` into `UnbillCore.xcframework` and generates the Swift bindings.
The generated project, Swift bindings, xcframework, `Info.plist`, and docs projection stay out of version control.

The screens follow the shared UI model adapted to Apple idioms.
A `NavigationSplitView` shell shows a ledger list and, on selection, ledger detail with people, conflicts, bills, and settlement.
Wired flows: create ledger; add and import known people; add a bill with Rust-computed settlement; resolve amendment conflicts;
create an invitation as a QR code; join by pasting or scanning a code; and a devices screen with manual per-peer sync.
Peer discovery and sync use the local network, so the app declares Local Network, Bonjour, and camera usage.

Two toolchains must cooperate to build for Apple.
The devenv shell repoints `DEVELOPER_DIR` and `SDKROOT` at a Nix Apple SDK and injects Nix compiler and linker variables.
`xcbuild.sh` scrubs them and selects the real Xcode for `xcodebuild` and `xcodegen`.
`build-rust.sh` links host artifacts with the Nix toolchain, then cross-compiles the iOS targets with Xcode's clang,
because the core pulls C crypto through `unbill-device` and no single global SDK satisfies both.
The app builds and runs on the iOS Simulator, Mac Catalyst, and a real iPhone with automatic code signing.
