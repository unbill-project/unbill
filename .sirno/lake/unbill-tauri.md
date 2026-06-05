---
core.desc: The desktop shell bridge and Tauri host for the native frontend.
core.name: Unbill Tauri
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
and trigger sync.
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

Most correctness testing belongs in core crates.
This crate is best verified by end-to-end UI flows and iOS project regeneration smoke tests.
