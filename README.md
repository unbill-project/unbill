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
- `apps/unbill-ui-native/` — default desktop UI for compact and multi-column layouts

That structure is deliberate. The core owns the rules. Shells adapt the core to different environments without becoming competing implementations of the product.

## If You’re Reading The Code

- [DESIGN.md](DESIGN.md) explains the system intent, model, and invariants.
- [IMPLEMENTATION.md](IMPLEMENTATION.md) explains how the workspace is put together.
- Crate and module `DESIGN.md` and `IMPLEMENTATION.md` files explain each local boundary.

The repo is design-first and test-first. Non-trivial changes are expected to update the relevant docs and land with tests close to the code they protect.

## Installation

Unbill is published through GitHub Releases, GHCR Docker images, AUR packages, and Homebrew formulas.

We support two installation types:

- User-friendly installation (prebuilt binaries/packages, no local compilation)
- Build-from-source installation (compile locally)

All commands below target the latest stable release only (GitHub "latest"), not pre-releases.

### 1) User-friendly installation (no source build)

#### macOS

- Install CLI binary:
  - `curl -L -o unbill-cli https://github.com/unbill-project/unbill/releases/latest/download/unbill-cli-macos-aarch64`
  - `chmod +x unbill-cli && sudo mv unbill-cli /usr/local/bin/unbill-cli`
- Install TUI binary:
  - `curl -L -o unbill-tui https://github.com/unbill-project/unbill/releases/latest/download/unbill-tui-macos-aarch64`
  - `chmod +x unbill-tui && sudo mv unbill-tui /usr/local/bin/unbill-tui`
- Homebrew:
  - `brew install unbill-project/tap/unbill-cli`
  - `brew install unbill-project/tap/unbill-tui`
- Desktop app: download and open `unbill-macos-aarch64.*` from Releases.

#### Windows

- Install CLI binary (PowerShell):
  - `Invoke-WebRequest https://github.com/unbill-project/unbill/releases/latest/download/unbill-cli-windows-x86_64.exe -OutFile unbill-cli.exe`
- Install TUI binary (PowerShell):
  - `Invoke-WebRequest https://github.com/unbill-project/unbill/releases/latest/download/unbill-tui-windows-x86_64.exe -OutFile unbill-tui.exe`
- Installer (direct link): `https://github.com/unbill-project/unbill/releases/latest/download/unbill-windows-x86_64.exe`
- Desktop app: if the installer filename changes by packaging format, open `https://github.com/unbill-project/unbill/releases/latest` and download the Windows installer asset there.

#### Linux

- Install CLI binary:
  - `curl -L -o unbill-cli https://github.com/unbill-project/unbill/releases/latest/download/unbill-cli-linux-x86_64`
  - `chmod +x unbill-cli && sudo mv unbill-cli /usr/local/bin/unbill-cli`
- Install TUI binary:
  - `curl -L -o unbill-tui https://github.com/unbill-project/unbill/releases/latest/download/unbill-tui-linux-x86_64`
  - `chmod +x unbill-tui && sudo mv unbill-tui /usr/local/bin/unbill-tui`
- Homebrew (Linuxbrew):
  - `brew install unbill-project/tap/unbill-cli`
  - `brew install unbill-project/tap/unbill-tui`
- AUR:
  - `yay -S unbill-cli-bin`
  - `yay -S unbill-tui-bin`
  - `yay -S unbill-bin`
- Desktop app artifact: download `unbill-linux-x86_64.*` from Releases.

#### Container

- Pull image: `docker pull ghcr.io/unbill-project/unbill-server:latest`
- Run image: `docker run --rm -p 8080:80 ghcr.io/unbill-project/unbill-server:latest`
- Version-pinned tag example: `docker pull ghcr.io/unbill-project/unbill-server:v0.1.0`

### 2) Build from source

Use source builds for development, custom modifications, or unreleased commits.

#### macOS / Windows / Linux

- Install Rust stable: `rustup toolchain install stable`.
- Build CLI and TUI from workspace root: `cargo build --release -p unbill-cli -p unbill-tui`.
- Run directly with Cargo during development:
  - `cargo run -p unbill-cli -- --help`
  - `cargo run -p unbill-tui -- --help`
- Desktop app build (after OS-specific Tauri prerequisites): `cargo tauri build --manifest-path crates/unbill-tauri/Cargo.toml`.

#### Container (build yourself)

- Build: `docker build -t unbill-server:local .`.
- Run: `docker run --rm -p 8080:80 unbill-server:local`.

### Development container option

For a reproducible source-oriented environment, use the repo’s `devenv.nix` and `devenv.yaml`.

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
