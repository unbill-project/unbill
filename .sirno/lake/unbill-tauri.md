---
core.desc: The desktop shell bridge and Tauri host for the native frontend.
core.name: Unbill Tauri
meta:
  frozen:
    - reviewed
core.category:
  - core.concept
core.belongs:
  - applications
  - frontend-ui
core.refines:
  - unbill-ui-native
---

`unbill-tauri` wires the desktop shell to `UnbillConsole`
and hosts the default desktop frontend.
It exposes async commands and frontend-ready DTOs.
It does not add business logic.

Commands bootstrap app state,
load ledger detail,
create or join ledgers,
add users,
save bills,
preview bill splits,
create invitations,
resolve conflicts,
trigger sync,
check for application updates,
and install application updates.
IDs cross the IPC boundary as strings and are parsed into typed Rust values before core code.

The desktop app owns one visible `main` window.
Capability bindings and frontend bootstrap assume that label remains stable.

The default frontend is `apps/unbill-ui-native`.
Tauri serves it through Trunk in development and loads built assets in release builds.
Frontend build and development commands are deterministic under Tauri's child-process environment.

On desktop builds the crate connects to a running `unbill-daemon`
through `RpcAsymChannel` and opens `UnbillConsole` over it.
On mobile builds it opens `LocalAsymChannel` with an in-process `FsStore` directly.

One shared `UnbillConsole` instance lives in Tauri state.
Command handlers stay async and return user-facing error strings.
The crate is an IPC boundary, not a domain layer.
The current boundary is command-first.
The internal service event stream is not yet the primary stable frontend contract of this crate.

`tauri.conf.json` is the source of truth for the desktop shell.
It points development to the native UI,
loads release assets,
normalizes Trunk color environment,
defines the visible `main` window,
and aligns with capability files.
The development server listens beyond loopback so iOS devices can reach the rewritten dev URL.

The same configuration owns the iOS project shape.
It points to a tracked XcodeGen template,
disables Xcode debug dylib support for debug device builds,
records the iOS development team,
and records native frameworks required by Rust dependencies.
Generated Xcode files are not hand-edited.

The command layer maps service state into frontend DTOs.
Bootstrap data includes known peer devices across local ledgers.
Ledger summaries include user display names so the frontend can show them
without loading full ledger detail.
Ledger detail includes only peer devices authorized for that ledger,
so the frontend can render ledger-scoped sync actions without recomputing authorization.

The crate registers `tauri-plugin-deep-link` to handle `unbill://` URLs.
On desktop, `tauri-plugin-single-instance` ensures only one instance runs.
When a second instance is launched with a deep link URL as a command-line argument,
the single-instance plugin forwards the arguments to the running instance,
which emits a `deep-link-open` event to the frontend.
On mobile, the deep-link plugin delivers URLs through the `on_open_url` callback.
Launch URLs from a cold start are stored in `PendingDeepLinks` state
and drained by the frontend via the `drain_pending_deep_links` command after mount.
`tauri.conf.json` declares the `unbill` custom scheme for desktop and mobile.
On Linux and Windows in debug builds, `register_all()` registers the scheme at runtime.
On macOS, scheme registration requires the bundled app installed in `/Applications`.

On Windows the crate registers the `tauri-plugin-updater` plugin at startup.
The updater checks for new versions against a `latest.json` hosted on the latest GitHub release.
`check_update` returns the available version or nothing.
`install_update` downloads and installs the update, then restarts the application.
On non-Windows desktop platforms the updater plugin dependency is compiled
but the plugin is not registered and the commands return no-ops.
The updater uses passive install mode on Windows so the NSIS installer
shows a progress bar without requiring user interaction.
`tauri.conf.json` declares `createUpdaterArtifacts: true`
and the `plugins.updater` section with a public key and GitHub release endpoint.
The capability file grants `updater:default` on all desktop platforms.

Most correctness testing belongs in core crates.
This crate is best verified by end-to-end UI flows and iOS project regeneration smoke tests.
