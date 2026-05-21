# Installing Unbill

Unbill ships a **desktop app**, **CLI**, **TUI**, and **sync daemon** for
macOS, Linux, and Windows, plus mobile builds for iOS and Android.
A Docker image is available for the relay server.

Pick your platform below.
Every section lists the quickest method first.

______________________________________________________________________

## macOS (Apple Silicon)

### Desktop app

**Homebrew cask** (recommended):

```sh
brew install --cask unbill-project/tap/unbill
```

Or download `unbill-macos-aarch64.dmg` from the
[latest release](https://github.com/unbill-project/unbill/releases/latest).

**Nix:**

```sh
cachix use unbill
nix profile install github:unbill-project/unbill#unbill-tauri
```

### CLI and TUI

**Homebrew:**

```sh
brew install unbill-project/tap/unbill-cli
brew install unbill-project/tap/unbill-tui
```

**Direct download** — grab the bare binaries from the
[latest release](https://github.com/unbill-project/unbill/releases/latest):

| Binary | File |
|--------|------|
| CLI | `unbill-cli-macos-aarch64` |
| TUI | `unbill-tui-macos-aarch64` |
| Daemon | `unbill-daemon-macos-aarch64` |

After downloading, make them executable and move to your PATH:

```sh
chmod +x unbill-cli-macos-aarch64
mv unbill-cli-macos-aarch64 /usr/local/bin/unbill-cli
```

**Nix:**

```sh
cachix use unbill
nix profile install github:unbill-project/unbill#unbill-cli
nix profile install github:unbill-project/unbill#unbill-tui
nix profile install github:unbill-project/unbill#unbill-daemon
```

______________________________________________________________________

## Linux (x86_64)

### Desktop app

**Download** from the
[latest release](https://github.com/unbill-project/unbill/releases/latest):

| Format | File |
|--------|------|
| AppImage | `unbill-linux-x86_64.AppImage` |
| Debian/Ubuntu | `unbill-linux-x86_64.deb` |
| Fedora/openSUSE | `unbill-linux-x86_64.rpm` |

For the AppImage:

```sh
chmod +x unbill-linux-x86_64.AppImage
./unbill-linux-x86_64.AppImage
```

For the `.deb`:

```sh
sudo dpkg -i unbill-linux-x86_64.deb
```

**Nix:**

```sh
cachix use unbill
nix profile install github:unbill-project/unbill#unbill-tauri
```

### CLI and TUI

**Homebrew:**

```sh
brew install unbill-project/tap/unbill-cli
brew install unbill-project/tap/unbill-tui
```

**AUR** (Arch Linux):

```sh
yay -S unbill-cli-bin unbill-tui-bin unbill-daemon-bin
```

**Direct download** — bare binaries from the
[latest release](https://github.com/unbill-project/unbill/releases/latest):

| Binary | File |
|--------|------|
| CLI | `unbill-cli-linux-x86_64` |
| TUI | `unbill-tui-linux-x86_64` |
| Daemon | `unbill-daemon-linux-x86_64` |

```sh
chmod +x unbill-cli-linux-x86_64
sudo mv unbill-cli-linux-x86_64 /usr/local/bin/unbill-cli
```

Tarballs (`.tar.gz`) are also available if you prefer.

**Nix:**

```sh
cachix use unbill
nix profile install github:unbill-project/unbill#unbill-cli
nix profile install github:unbill-project/unbill#unbill-tui
nix profile install github:unbill-project/unbill#unbill-daemon
```

______________________________________________________________________

## Windows (x86_64)

### Desktop app

Download `unbill-windows-x86_64.msi` from the
[latest release](https://github.com/unbill-project/unbill/releases/latest)
and run the installer.

A portable `.exe` (`unbill-windows-x86_64.exe`) is also available.

### CLI and TUI

Download from the
[latest release](https://github.com/unbill-project/unbill/releases/latest):

| Binary | File |
|--------|------|
| CLI | `unbill-cli-windows-x86_64.exe` |
| TUI | `unbill-tui-windows-x86_64.exe` |
| Daemon | `unbill-daemon-windows-x86_64.exe` |

Place them somewhere on your `PATH`, or run directly.

______________________________________________________________________

## iOS

Unbill is distributed for iOS via **AltStore** sideloading.

1. Install [AltStore](https://altstore.io) on your device.
1. In AltStore, go to **Sources** and add:
   ```
   https://raw.githubusercontent.com/unbill-project/unbill/main/altstore-source.json
   ```
1. Browse the source and install **unbill**.

Requires iOS 14.0 or later.
The IPA is unsigned — AltStore re-signs it with your Apple ID.

______________________________________________________________________

## Android

Download `unbill-android-universal.apk` from the
[latest release](https://github.com/unbill-project/unbill/releases/latest).

Enable "Install from unknown sources" in your device settings, then open the APK.

______________________________________________________________________

## Server (Docker)

The relay server is available as a Docker image on GHCR.

```sh
docker pull ghcr.io/unbill-project/unbill-server:latest
docker run --rm -p 8080:80 ghcr.io/unbill-project/unbill-server:latest
```

Or build locally from the repository:

```sh
docker build -t unbill-server:local .
docker run --rm -p 8080:80 unbill-server:local
```

______________________________________________________________________

## Nix flake integration

For NixOS or nix-darwin, add unbill as a flake input:

```nix
# flake.nix
{
  inputs.unbill = {
    url = "github:unbill-project/unbill/main";
    inputs.nixpkgs.follows = "nixpkgs";
  };
}
```

Configure the binary cache so Nix fetches pre-built binaries:

```nix
nix.settings = {
  substituters = [ "https://unbill.cachix.org" ];
  trusted-public-keys = [ "unbill.cachix.org-1:157H1n8eC+rAITRruhXXuS5CUWvSgUIhkzRIbp+AKng=" ];
};
```

Expose packages via an overlay and add to `environment.systemPackages`
or home-manager's `home.packages`:

```nix
unbillOverlay = _: _: {
  inherit (unbill.packages.${system})
    unbill-cli unbill-tui unbill-daemon unbill-tauri;
};
```

Available packages: `unbill-cli`, `unbill-tui`, `unbill-daemon`, `unbill-tauri`.

______________________________________________________________________

## Build from source

Requires Rust stable.

```sh
git clone https://github.com/unbill-project/unbill.git
cd unbill
cargo build --release -p unbill-cli -p unbill-tui -p unbill-daemon
```

Binaries are in `target/release/`.

The desktop app requires [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
for your platform, then:

```sh
cargo tauri build --manifest-path crates/unbill-tauri/Cargo.toml
```
