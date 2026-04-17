# unbill

A fully decentralized, peer-to-peer, offline-first bill-splitting application.
Think of it as "Splitwise without servers."

> **Status:** Early development — Milestone M0 (workspace skeleton).
> Nothing is user-facing yet. See the [roadmap](#roadmap) below.

## What is it?

A group of friends shares a **ledger**: a history of expenses, who paid, and who owes whom.
Each participant runs unbill on their own device. **There is no central server.**
Devices sync directly with each other over P2P networking (Iroh/QUIC) when online;
changes made while offline propagate automatically once connectivity returns.

See [DESIGN.md](DESIGN.md) for the full architecture and design rationale.

## Non-goals

- Not a payment processor — unbill records who owes whom; it does not move money.
- Not a commercial service — no accounts, no subscriptions, no telemetry.
- Not a general-purpose accounting tool.

## Repository layout

```
unbill/
├── crates/
│   ├── unbill-core/     # The library: CRDT ledger, persistence, P2P sync
│   ├── unbill-cli/      # Command-line frontend
│   └── unbill-tauri/    # Tauri desktop/mobile backend
└── apps/
    └── unbill-desktop/  # React frontend (Vite + TanStack Query + Tailwind)
```

## Building

**Prerequisites:** Rust (stable), Cargo.

```bash
# Check / build the core library and CLI
cargo build -p unbill-core -p unbill-cli

# Run tests
cargo test -p unbill-core

# Run the CLI (stub — commands are not yet implemented)
cargo run -p unbill-cli -- --help
```

The `unbill-tauri` crate additionally requires GTK and WebKit system libraries.
On Ubuntu/Debian:

```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf
cargo build  # full workspace
```

The JavaScript frontend requires Node.js ≥ 20 and pnpm ≥ 9:

```bash
pnpm install
pnpm dev     # Vite dev server on :1420
```

## Roadmap

| Milestone | Description | Status |
|-----------|-------------|--------|
| M0 | Workspace skeleton, DESIGN.md, stub crates, tests | **Done** |
| M1 | Core data model: `LedgerDoc`, `add_bill`, `list_bills` | Planned |
| M2 | Persistence: `SqliteStore`, `UnbillService`, CLI commands | Planned |
| M3 | Offline file sync: export/import `.unbill` files | Planned |
| M4 | P2P sync: Iroh integration, invite flow | Planned |
| M5 | Desktop GUI: Tauri + React frontend | Planned |

## Design philosophy

This project is **design-first and test-first**. Every non-trivial module begins
with a `DESIGN.md` before any production code, and tests are written before the
implementation. See [DESIGN.md §0](DESIGN.md#0-preamble-design-first-test-first-philosophy)
for the full rationale.

## License

MIT — see [LICENSE](LICENSE).
