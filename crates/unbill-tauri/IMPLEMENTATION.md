# unbill-tauri — Implementation

`src/lib.rs` defines the DTOs, command handlers, and bootstrap flow. Tauri setup opens `FsStore`, constructs `UnbillService`, and shares it through `tauri::State`.

Most correctness testing belongs in `unbill-core`. This crate is best verified through end-to-end UI flows that exercise the full Tauri boundary.
