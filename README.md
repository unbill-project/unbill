# Unbill

<!-- Compiled from unbill-docs. Update the Sirno lake first, then refresh this artifact. -->

<!-- sirno:witness:unbill:begin -->

Offline-first bill splitting for small trusted groups.

Unbill keeps shared expense ledgers on member devices and syncs them peer-to-peer.
There is no hosted source of truth,
hosted account system,
or telemetry surface.
The app records who paid and who owes whom.
It does not move money.

Unbill is meant for households, trips, couples,
and other small groups that already trust each other.
It is not a payment network,
bank integration layer,
general accounting package,
or product for hostile or anonymous groups.

<!-- sirno:witness:unbill:end -->

## Design Source

<!-- sirno:witness:compiled-markdown-artifacts:begin -->

The project design lives in [unbill-docs](unbill-docs).
Read [unbill-docs/introduction.md](unbill-docs/introduction.md) first.
The old Markdown documentation was lowered into the lake and audited in
[unbill-docs/documentation-coverage.md](unbill-docs/documentation-coverage.md).
Recovered diagrams are tracked in
[unbill-docs/visual-diagram-audit.md](unbill-docs/visual-diagram-audit.md).

<!-- sirno:witness:compiled-markdown-artifacts:end -->

## Repository Shape

<!-- sirno:witness:workspace-layout:begin -->

Unbill is a Rust workspace built from focused crates and thin applications.

- `crates/unbill-model` holds domain data types.
- `crates/unbill-storage` owns Automerge documents and store traits.
- `crates/unbill-store-fs` and `crates/unbill-store-memory` provide store backends.
- `crates/unbill-device` owns the device role.
- `crates/unbill-console` owns the console-side service projection.
- `crates/unbill-symmetric-channel` owns device-to-device Iroh sync and join.
- `crates/unbill-asymmetric-channel` owns device-to-console transports.
- `crates/unbill-tauri` and `crates/unbill-ui-components` support the desktop and web UI.
- `apps` contains the CLI, TUI, daemon, server, native UI, and remote UI.

<!-- sirno:witness:workspace-layout:end -->

## Build And Run

Use Rust stable.

```sh
cargo build --workspace
cargo test --workspace --exclude unbill-tauri
```

Run local tools through Cargo:

```sh
cargo run -p unbill-daemon
cargo run -p unbill-cli -- --help
cargo run -p unbill-tui -- --help
```

Build the desktop shell after installing platform-specific Tauri prerequisites:

```sh
cargo tauri build --manifest-path crates/unbill-tauri/Cargo.toml
```

Build the server container locally:

```sh
docker build -t unbill-server:local .
docker run --rm -p 8080:80 unbill-server:local
```

## Distribution

<!-- sirno:witness:distribution-and-release:begin -->

Unbill publishes prebuilt CLI and TUI binaries,
desktop app artifacts,
GHCR Docker images,
Homebrew formulae,
and AUR packages.
Source builds can also use the Nix flake or the repository `devenv` files.

Releases are managed by `cargo release`.
The version pipeline builds artifacts,
publishes GitHub releases,
updates AUR packages,
updates Homebrew formulae,
and pushes the server image to GHCR.

## License

Unbill is licensed under either Apache-2.0 or MIT, at your option.
See [LICENSE](LICENSE),
[LICENSE-APACHE](LICENSE-APACHE),
and [LICENSE-MIT](LICENSE-MIT).

<!-- sirno:witness:distribution-and-release:end -->
