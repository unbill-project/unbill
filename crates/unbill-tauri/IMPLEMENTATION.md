# Unbill Tauri — Implementation

`src/lib.rs` defines the DTOs, command handlers, and bootstrap flow. Tauri setup opens `FsStore`, constructs `UnbillConsole`, and shares it through `tauri::State`.

`tauri.conf.json` is the source of truth for the desktop shell. It starts the Leptos frontend from `../../apps/unbill-ui-native` in development, loads the built `dist/` output in release builds, normalizes Trunk's color environment, and defines the single visible `main` window used by the default capability set. The development server listens beyond loopback so iOS devices can reach the rewritten development URL.

The same configuration also owns the iOS project shape. It points Tauri at `ios/project.yml`, a custom XcodeGen template derived from Tauri's default iOS template. The template disables Xcode Debug Dylib Support so debug device builds link the Rust `libapp.a` static library into the normal app binary instead of a separate `unbill.debug.dylib`. The iOS development team and extra native frameworks are recorded in Tauri config so regenerating the project preserves signing and linker inputs.

The Tauri command layer maps core service state into frontend DTOs. Bootstrap data includes all known peer devices across local ledgers, while ledger detail includes only the peer devices authorized for that ledger so the frontend can render ledger-scoped sync actions without recomputing authorization locally.

Most correctness testing belongs in `unbill-core`. This crate is best verified through end-to-end UI flows that exercise the full Tauri boundary. iOS project configuration is smoke-tested by regenerating the mobile project and checking the generated Xcode project contains the tracked debug dylib override.
