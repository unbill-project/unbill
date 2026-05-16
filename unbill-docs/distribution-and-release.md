---
name: Distribution And Release
desc: The installation channels and release mechanics described by the repository.
category:
  - concept
belongs:
  - unbill
---

Unbill publishes user-friendly installation artifacts and source-build paths.
The repository is licensed under MIT in `LICENSE`.

User-friendly installation includes prebuilt CLI and TUI binaries,
desktop app artifacts,
GHCR Docker images,
Homebrew formulas,
and AUR packages.
Release links target the latest stable GitHub release rather than prereleases.

Source builds use Rust stable from the workspace root.
The workspace builds the CLI and TUI as release binaries,
and the Tauri desktop app builds through the Tauri manifest in `crates/unbill-tauri`.

The Docker server can be pulled from GHCR or built locally from the repository Dockerfile.
The example deployment includes Compose configuration for a persistent server volume.

Nix users can install flake packages directly from the repository or build them locally.
Development environments can use `devenv.nix` and `devenv.yaml`.

Releases are managed by `cargo release`.
The release flow bumps workspace and Tauri versions,
commits the change,
creates a `v{version}` tag,
and relies on the version-tag CI pipeline.
Dry run is the default and execution must be explicit.

Current repository status:
the Rust model, storage, console, device, channel crates,
CLI, TUI, daemon, server, Tauri boundary,
and Leptos native and remote frontends exist in the workspace.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from):
  - [ci-cd-pipeline](ci-cd-pipeline.md)

> **Sirno generated links end.**
