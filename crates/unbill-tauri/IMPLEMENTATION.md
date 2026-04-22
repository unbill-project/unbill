# Unbill Tauri — Implementation

`src/lib.rs` defines the DTOs, command handlers, and bootstrap flow. Tauri setup opens `FsStore`, constructs `UnbillService`, and shares it through `tauri::State`.

`tauri.conf.json` is the source of truth for the desktop shell. It starts the Leptos frontend from `../../apps/unbill-ui-leptos` in development, loads the built `dist/` output in release builds, and defines the single visible `main` window used by the default capability set.

Most correctness testing belongs in `unbill-core`. This crate is best verified through end-to-end UI flows that exercise the full Tauri boundary.
