# Unbill

<p align="center">
  <img src="./unbill-icon.svg" alt="Unbill zipper logo" width="160">
</p>

Offline-first bill splitting for small trusted groups.

Think of unbill as shared expense tracking that does not depend on a company staying alive.

Each ledger lives on member devices and syncs directly between them. There is no hosted source of truth, no account system, and no telemetry. The app records who paid and who owes whom. It does not move money.

## Why This Exists

Splitting expenses is usually treated as a hosted service problem: sign up, trust the server, hope the product stays around, and accept that your group's data lives somewhere else.

Unbill takes the opposite approach. It starts from a simple idea:

- expense tracking should still work when you are offline
- your group should keep its own data
- sync should happen between devices, not through a permanent central owner
- the system should stay understandable enough that the whole codebase can be reasoned about

That makes unbill a good fit for households, trips, couples, and other small groups that already trust each other and just want one durable shared record.

## What Kind Of Project This Is

Unbill is intentionally narrow.

- It is not a payment network.
- It is not a bank integration layer.
- It is not a general accounting package.
- It is not designed for hostile or anonymous groups.

The goal is not to cover every edge of personal finance. The goal is to make shared expense tracking durable, local-first, and easy to trust.

## How It Works

Each group uses a ledger. That ledger contains users, authorized devices, and bills. Devices can create and amend bills while offline, then sync later when they can reach each other again.

At the data level, the system prefers append-only shared history and deterministic projection over mutable central state. At the product level, that means unbill tries to behave like a tool your group keeps, not a service you rent.

## Repository Shape

The repository is centered on one Rust core and a few thin adapters around it.

- `crates/unbill-core/` — the domain engine: ledger model, storage, sync, settlement, service API
- `crates/unbill-cli/` — command-line frontend for scripting, dogfooding, and end-to-end verification
- `crates/unbill-tauri/` — Tauri bridge and desktop shell host for the Rust core
- `apps/unbill-ui-leptos/` — default desktop UI for compact and multi-column layouts

That structure is deliberate. The core owns the rules. Shells adapt the core to different environments without becoming competing implementations of the product.

## If You’re Reading The Code

- [DESIGN.md](DESIGN.md) explains the system intent, model, and invariants.
- [IMPLEMENTATION.md](IMPLEMENTATION.md) explains how the workspace is put together.
- Crate and module `DESIGN.md` and `IMPLEMENTATION.md` files explain each local boundary.

The repo is design-first and test-first. Non-trivial changes are expected to update the relevant docs and land with tests close to the code they protect.

## Releasing

Releases are managed with `cargo release`. Run from the workspace root.

```sh
cargo release patch   # 0.1.0 → 0.1.1
cargo release minor   # 0.1.0 → 0.2.0
cargo release major   # 0.1.0 → 1.0.0
cargo release 1.2.3   # exact version
```

Each command bumps the version in `Cargo.toml` and `tauri.conf.json`, commits the change, creates a `v{version}` tag, and pushes. The version release CI pipeline triggers automatically on the tag push.

Dry run is the default. Pass `--execute` to actually perform the release:

```sh
cargo release patch --execute
```

## Status

The Rust core, CLI, sync layer, Tauri boundary, and Leptos desktop UI exist today.

## License

MIT. See [LICENSE](LICENSE).
