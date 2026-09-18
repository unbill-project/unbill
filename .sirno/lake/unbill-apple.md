---
core.desc: The native SwiftUI frontend for Apple platforms and the macOS release app.
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
It supplies the macOS release app, replacing the Tauri macOS bundle in the release pipeline.
Tauri continues to supply the Linux and Windows desktop bundles.
It targets iPhone and iPad as one universal build and macOS through Mac Catalyst.

The app is backed by the real Rust core.
`ConsoleClient` is the single Swift boundary, an async protocol over the `UnbillConsole` orchestration surface.
`RustConsoleClient` is its only implementation: an actor that calls `unbill-ffi` and runs the synchronous bridge off the main thread.
There is no mock backend.
Mac Catalyst acquires the exclusive filesystem storage lock.
The daemon uses a separate SQLite database without that directory lock.
If another filesystem backend owns the directory, startup displays a data-directory-in-use message
with the path and instructions to close the other backend and reopen the app.
Other startup failures display their error instead of crashing.
The new-ledger currency picker obtains the complete Rust currency catalog through `ConsoleClient`
and UniFFI, sorted by currency code, rather than maintaining a Swift shortlist.

`unbill-ffi` is a Cargo workspace member that bridges `UnbillConsole` to Swift with UniFFI.
It mirrors the aggregated command granularity of the Tauri bridge rather than the raw console methods:
bootstrap, ledger detail, create ledger, create and import users, save bill, resolve conflict,
create invitation, join, sync, and a `ServiceEvent` callback stream.
The DTO assembly is ported from `unbill-tauri`; a shared `unbill-shell` crate would remove that duplication.

The project lives at `apps/unbill-apple`, outside the Cargo workspace.
An `xcodegen` `project.yml` defines one application target for iOS device, simulator, and Mac Catalyst.
It declares a shared `unbill` scheme so fresh CI checkouts can build without opening Xcode.
`build-rust.sh` compiles `unbill-ffi` into `UnbillCore.xcframework` and generates the Swift bindings.
The generated project, Swift bindings, xcframework, `Info.plist`, and docs projection stay out of version control.

`build-apple-dmg.yml` builds an Apple Silicon Catalyst DMG on macOS CI.
It pins Xcode 26.6 on the macOS latest runner for both Rust and Swift builds.
Building against the 26-series SDKs enables the system's Liquid Glass design on macOS Tahoe;
the deployment target remains compatible with older macOS releases.
It is reusable-only, with read-only repository permissions and no standalone event triggers.
Only nightly and version-tag releases enable its opt-in job in the shared build workflow.
Direct manual runs of the shared build workflow do not build the DMG.
The Rust build's `--catalyst-only` option builds only the Mac slice for this workflow;
the default build still includes iOS device and simulator slices.
The workflow uploads `unbill-macos-aarch64.dmg` in a `binaries-*` artifact for the existing release collector.
It packages `Unbill.app` to preserve the Homebrew cask contract and takes its version from the Rust workspace.
The app is ad-hoc signed; Developer ID signing and notarization are not configured.

The screens follow the shared UI model adapted to Apple idioms.
A `NavigationSplitView` shell shows a ledger list and, on selection, ledger detail with people, conflicts, bills, and settlement.
Wired flows: create ledger; add and import known people; add a bill with Rust-computed settlement; resolve amendment conflicts;
create an invitation as a QR code; join by pasting or scanning a code; and a devices screen with manual per-peer sync.
Live QR scanning uses VisionKit only on supported iOS hardware.
Mac Catalyst builds exclude the scanner and its presentation; joining on Mac uses invitation-link entry.
Peer discovery and sync use the local network, so the app declares Local Network, Bonjour, and camera usage.

Two toolchains must cooperate to build for Apple.
The devenv shell repoints `DEVELOPER_DIR` and `SDKROOT` at a Nix Apple SDK and injects Nix compiler and linker variables.
`xcbuild.sh` scrubs them and selects the real Xcode for `xcodebuild` and `xcodegen`.
`build-rust.sh` links host artifacts with the Nix toolchain, then cross-compiles the iOS targets with Xcode's clang,
because the core pulls C crypto through `unbill-device` and no single global SDK satisfies both.
Swift binding generation invokes the host binary built in the first phase directly,
so it does not rebuild host dependencies under the cross-compilation environment.
The app builds and runs on the iOS Simulator, Mac Catalyst, and a real iPhone with automatic code signing.
