# unbill

Decentralized, offline-first, peer-to-peer bill splitting for small trusted groups.
Think of it as "Splitwise without servers."

Ledgers live on member devices and sync directly between them. There is no server, account system, or hosted source of truth.
Unbill records obligations only. It does not move money, act as bookkeeping software, or collect telemetry.

## What It Is

Unbill is for situations like a household, a trip, or a recurring shared budget where a few people need one durable record of who paid and who owes what. Each shared group uses a ledger. Every device with access to that ledger keeps a local copy and can continue working while offline.

The project is intentionally narrow. It prefers a small, understandable model over trying to cover every accounting or fintech use case. The goal is dependable shared expense tracking that still works when a network, company, or account system is missing.

## What It Is Not

- not a payment network
- not a bank integration layer
- not a general accounting package
- not a product aimed at hostile or anonymous groups

## How The System Hangs Together

At the center is `unbill-core`, which owns the ledger model, Automerge document, persistence, sync, settlement, and the service API used by shells. Other workspace members are adapters around that core:

- the CLI exercises the service directly for scripting and end-to-end verification
- the Tauri crate exposes the service through desktop or mobile IPC
- the UI apps consume typed DTOs and present local interaction flows

That structure is deliberate. The codebase treats the Rust core as the source of truth and keeps UI code focused on presentation and short-lived client state.

## Workspace

- `crates/unbill-core/` — ledger model, Automerge document, storage, sync, settlement, service facade
- `crates/unbill-cli/` — command-line frontend over the core service
- `crates/unbill-tauri/` — Tauri bridge between Rust and desktop or mobile UI
- `apps/unbill-ui-leptos/` — shared Leptos UI for compact and multi-column shells
- `apps/unbill-desktop/` — early React desktop shell

## Docs

- [DESIGN.md](DESIGN.md) — system intent and core invariants
- [IMPLEMENTATION.md](IMPLEMENTATION.md) — workspace and runtime structure
- crate and module `DESIGN.md` / `IMPLEMENTATION.md` files — local contracts

The root docs describe the whole system. Crate and module docs narrow that view to one boundary at a time.

## Status

The Rust core, CLI, sync layer, and Tauri boundary exist. UI work is split between an active Leptos app and an early React shell.

## Working Style

This repository is design-first and test-first. Non-trivial changes are expected to update the relevant `DESIGN.md` and `IMPLEMENTATION.md` files, then land with tests close to the code they protect. The docs are meant to stay brief, current, and architectural rather than becoming changelogs.

## License

MIT. See [LICENSE](LICENSE).
