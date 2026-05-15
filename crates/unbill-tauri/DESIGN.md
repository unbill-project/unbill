# Unbill Tauri

Tauri bridge that wires the desktop shell to `UnbillConsole` and the default desktop shell for Unbill. It exposes async commands and frontend-ready DTOs without adding new business logic.

## Contract

- commands bootstrap app state, load ledger detail, create or join ledgers, add users, save bills, create invitations, and trigger sync
- IDs cross the boundary as strings and are parsed back into typed Rust values before touching core code
- the desktop app owns exactly one visible `main` window; capability bindings and frontend bootstrap both assume that label remains stable
- the default desktop frontend is `apps/unbill-ui-native`, served by Trunk in development and loaded from its built assets in release builds
- frontend build and development commands are deterministic under the environment Tauri passes to child processes
- the generated iOS project is reproducible from tracked Tauri configuration and a tracked XcodeGen template
- iOS debug builds link the Rust static library into the app executable instead of Xcode's separate debug dylib layout
- iOS native frameworks required by Rust dependencies are declared in Tauri configuration

The current boundary is command-first. `UnbillConsole` has an internal `ServiceEvent` stream, but a stable frontend event contract is not yet the primary design surface of this crate.

## Rules

- one shared `UnbillConsole` (from `unbill-console`) instance lives in Tauri state; it will migrate to a `LocalAsymChannel` as the asym channel layer matures
- command handlers stay async and return user-facing error strings
- this crate is an IPC boundary, not a domain layer
- Tauri config stays aligned with the capability files: the window label used by capabilities must exist in `tauri.conf.json`
- iOS signing and Xcode build settings must be configured through Tauri inputs, not by editing ignored generated Xcode files by hand
