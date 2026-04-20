# unbill-tauri

Tauri bridge around `UnbillService`. It exposes async commands and frontend-ready DTOs without adding new business logic.

## Contract

- commands bootstrap app state, load ledger detail, create or join ledgers, add users, save bills, create invitations, and trigger sync
- IDs cross the boundary as strings and are parsed back into typed Rust values before touching core code

The current boundary is command-first. `UnbillService` has an internal `ServiceEvent` stream, but a stable frontend event contract is not yet the primary design surface of this crate.

## Rules

- one shared `UnbillService` instance lives in Tauri state
- command handlers stay async and return user-facing error strings
- this crate is an IPC boundary, not a domain layer
