---
name: Distribution And Release
desc: The installation channels and release mechanics described by the repository.
category:
  - concept
belongs:
  - unbill
---

Unbill publishes user-friendly installation artifacts and source-build paths.
Unbill is dual licensed as `MIT OR Apache-2.0`.
Workspace package metadata declares that expression,
the AUR templates declare both licenses,
and the repository root includes `LICENSE`, `LICENSE-MIT`, and `LICENSE-APACHE`.

User-friendly installation includes prebuilt CLI and TUI binaries,
desktop app artifacts,
GHCR Docker images,
Homebrew formulas,
AUR packages,
and an AltStore source for iOS sideloading.
Release links target the latest stable GitHub release rather than prereleases.

Source builds use Rust stable from the workspace root.
The workspace builds the CLI and TUI as release binaries,
and the Tauri desktop app builds through the Tauri manifest in `crates/unbill-tauri`.

The Docker server can be pulled from GHCR or built locally from the repository Dockerfile.
The example deployment includes Compose configuration for a persistent server volume.

## Nix

Nix users can install flake packages directly from the repository.
Pre-built binaries are served via Cachix to avoid local compilation.

### Quick install (single user)

```bash
# Add the binary cache (one-time)
cachix use unbill

# Install a package
nix profile install github:unbill-project/unbill#unbill-tauri
nix profile install github:unbill-project/unbill#unbill-cli
nix profile install github:unbill-project/unbill#unbill-tui
nix profile install github:unbill-project/unbill#unbill-daemon
```

### NixOS / nix-darwin flake integration

Add unbill as a flake input and configure the binary cache:

```nix
# flake.nix inputs
unbill = {
  url = "github:unbill-project/unbill/main";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

Add the Cachix substituter so Nix fetches pre-built binaries:

```nix
# In your NixOS or nix-darwin configuration
nix.settings = {
  substituters = [ "https://unbill.cachix.org" ];
  trusted-public-keys = [ "unbill.cachix.org-1:157H1n8eC+rAITRruhXXuS5CUWvSgUIhkzRIbp+AKng=" ];
};
```

Expose the packages via an overlay:

```nix
# In your flake outputs
unbillOverlay = _: _: {
  inherit (unbill.packages.${system})
    unbill-cli unbill-tui unbill-daemon unbill-tauri;
};
```

Then add the packages to `environment.systemPackages` or home-manager's `home.packages`.

### Available flake packages

- `unbill-cli` — command-line interface
- `unbill-tui` — terminal UI
- `unbill-daemon` — background sync daemon
- `unbill-tauri` — desktop app (includes .desktop entry for app launchers)

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
  - [altstore-source](altstore-source.md)
  - [ci-cd-pipeline](ci-cd-pipeline.md)

> **Sirno generated links end.**
