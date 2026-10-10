---
core.desc: The installation channels and release mechanics described by the repository.
core.name: Distribution And Release
meta:
  frozen:
    - reviewed
core.category:
  - core.concept
core.belongs:
  - unbill
---

Unbill publishes user-friendly installation artifacts and source-build paths.
Unbill is dual licensed as `MIT OR Apache-2.0`.
Workspace package metadata declares that expression,
the AUR templates declare both licenses,
and the repository root includes `LICENSE`, `LICENSE-MIT`, and `LICENSE-APACHE`.

`INSTALL.md` at the repository root is the per-platform installation guide.
It covers macOS, Linux, Windows, iOS, Android, and Docker,
listing every available method for each platform.
`README.md` links to it from a condensed Install section.

User-friendly installation includes:

- **Direct binary downloads** attached to each GitHub release
  (CLI, TUI, daemon for Linux x86_64, macOS aarch64, Windows x86_64).
- **Desktop app installers** per platform
  (.dmg for macOS, .deb/.rpm/.AppImage for Linux, .msi/.exe for Windows).
- **Homebrew** formulas for CLI, TUI, and daemon, plus a cask for the macOS desktop app.
- **AUR** binary packages for Arch Linux (CLI, TUI, daemon, desktop app).
- **Nix** flake packages via Cachix (CLI, TUI, daemon, desktop app).
- **AltStore source** for iOS sideloading.
- **Android APK** attached to each GitHub release.
- **GHCR Docker image** for the relay server.
- **Windows auto-updater** via `tauri-plugin-updater`.
  The Windows desktop app checks for updates on startup
  against `latest.json` hosted on the latest GitHub release.
  Users see a banner and can trigger download and install from the UI.
  The NSIS installer runs in passive mode (progress bar, no interaction).

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
The release flow bumps application version sources,
commits the change,
creates a `v{version}` tag,
and relies on the version-tag CI pipeline.
Dry run is the default and execution must be explicit.

Application version sources that must stay in sync on every release:

1. `Cargo.toml` workspace `version` — the canonical Rust version.
2. `crates/unbill-tauri/tauri.conf.json` `version` — becomes `CFBundleShortVersionString` in the iOS IPA.
   Tauri 2 does not inherit `version.workspace = true` for iOS builds.
3. `apps/unbill-apple/project.yml` `MARKETING_VERSION` — the local Swift Apple app default.
   CI supplies the workspace version when building release apps.

After the IPA is published, update `altstore-source.json` from the actual asset,
including its version, bundle identifier, pinned URL, minimum iOS version,
permissions, release date, and exact size. The source lists only the native Swift Apple app as Unbill.

AltStore verifies that the source JSON `version` exactly matches
`CFBundleShortVersionString` in the IPA and refuses to install on mismatch.

Current repository status:
the Rust model, storage, console, device, channel crates,
CLI, TUI, daemon, server, Tauri boundary,
and Leptos native and remote frontends exist in the workspace.

The Homebrew daemon formula supports a per-user `brew services start unbill-daemon`
service, using the same default data directory as desktop clients. It does not
start automatically on installation. Stable version releases update this formula
alongside CLI and TUI; nightly releases do not update the Homebrew tap.

Homebrew CLI, TUI, and Apple App packages depend on the daemon formula.
Installation does not start the service. Caveats explain that an existing
Nix-managed daemon can be used instead of starting a second daemon.

The Swift Apple iOS app is built unsigned by `build-apple-ios.yml` on macOS CI.
Nightly and version releases enable `build_apple_ios`; manual builds can opt in.
The Rust build script's `--ios-only` option builds the device slice and generates
Swift bindings. Xcode builds Release for generic iOS with signing disabled.
The workflow packages `Payload/unbill.app` as `unbill-apple-ios.ipa` in a
`binaries-apple-ios-aarch64` artifact, collected by the GitHub release workflow.
Users sign the IPA themselves for sideloading. No signing secrets are required.
