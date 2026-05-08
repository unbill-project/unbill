{
  description = "Unbill — shared bill tracking";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # wasm-bindgen-cli built at the exact version used in Cargo.lock.
        # Nixpkgs often lags; building from source ensures the versions match.
        wasm-bindgen-cli = pkgs.rustPlatform.buildRustPackage rec {
          pname = "wasm-bindgen-cli";
          version = "0.2.118";
          src = pkgs.fetchCrate {
            inherit pname version;
            hash = "sha256-ve783oYH0TGv8Z8lIPdGjItzeLDQLOT5uv/jbFOlZpI=";
          };
          cargoHash = "sha256-EYDfuBlH3zmTxACBL+sjicRna84CvoesKSQVcYiG9P0=";
        };

        # Host toolchain for CLI / TUI / Tauri binary.
        craneLib = crane.mkLib pkgs;

        # Stable Rust with wasm32 target for the Leptos frontend.
        rustWithWasm = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
        craneLibWasm = (crane.mkLib pkgs).overrideToolchain rustWithWasm;

        commonArgs = {
          pname = "unbill";
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          # Only resolve deps for the two packages we care about, avoiding
          # gtk/glib/webkit2gtk from unbill-tauri.
          cargoExtraArgs = "-p unbill-cli -p unbill-tui";
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        };

        # Build dependencies for the workspace minus tauri, shared between packages.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        unbill-cli = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--bin unbill-cli";
            doCheck = false;
          }
        );

        unbill-tui = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--bin unbill-tui";
            doCheck = false;
          }
        );

        # ── Desktop (Tauri) ──────────────────────────────────────────────────

        # Step 1: build the Leptos/WASM frontend with trunk.
        #
        # Deps are built separately with -p unbill-ui-native to avoid
        # compiling mio/tokio and other crates that don't support wasm32.
        # The trunk invocation then uses those pre-built artifacts and is
        # pointed at the app via --config so it runs correctly from the
        # workspace root.
        # Source for the WASM frontend: Rust sources + HTML/CSS/assets that
        # trunk needs (index.html, style.css, Trunk.toml, component styles).
        uiSrc = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            let
              relPath = pkgs.lib.removePrefix (toString ./. + "/") path;
            in
            (craneLibWasm.filterCargoSources path type)
            || (pkgs.lib.hasPrefix "apps/unbill-ui-native" relPath)
            || (pkgs.lib.hasPrefix "crates/unbill-ui-components" relPath);
        };

        uiWasmArgs = {
          pname = "unbill-ui-native";
          src = uiSrc;
          cargoExtraArgs = "-p unbill-ui-native";
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          nativeBuildInputs = [
            wasm-bindgen-cli
            pkgs.binaryen
          ];
        };

        uiArtifacts = craneLibWasm.buildDepsOnly uiWasmArgs;

        unbill-ui-dist = craneLibWasm.buildTrunkPackage {
          pname = "unbill-ui-native";
          src = uiSrc;
          # trunkIndexPath is resolved relative to the --config working dir
          # (apps/unbill-ui-native/), so just "index.html" is correct here.
          trunkIndexPath = "index.html";
          trunkExtraArgs = "--config apps/unbill-ui-native/Trunk.toml";
          cargoArtifacts = uiArtifacts;
          wasm-bindgen-cli = wasm-bindgen-cli;
          nativeBuildInputs = [
            pkgs.trunk
            wasm-bindgen-cli
            pkgs.binaryen
          ];
          # crane derives the install path from trunkIndexPath ("." here),
          # so point it explicitly at where trunk actually writes the dist.
          installPhaseCommand = "cp -r apps/unbill-ui-native/dist $out";
        };

        # Step 2: build the Tauri binary, injecting the frontend dist before
        # tauri-build embeds it into the binary.
        #
        # Source filter uses relative paths so hasPrefix works correctly
        # against the Nix store copy of the source.
        tauriSrc = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            let
              relPath = pkgs.lib.removePrefix (toString ./. + "/") path;
            in
            (craneLib.filterCargoSources path type)
            || (pkgs.lib.hasPrefix "crates/unbill-tauri" relPath);
        };

        tauriArgs = {
          pname = "unbill-desktop";
          src = tauriSrc;
          strictDeps = true;
          nativeBuildInputs =
            pkgs.lib.optionals pkgs.stdenv.isLinux (
              with pkgs;
              [
                pkg-config
                wrapGAppsHook3
                gobject-introspection
              ]
            );
          buildInputs =
            pkgs.lib.optionals pkgs.stdenv.isLinux (
              with pkgs;
              [
                at-spi2-atk
                cairo
                gdk-pixbuf
                glib
                gtk3
                harfbuzz
                libsoup_3
                openssl
                pango
                webkitgtk_4_1
              ]
            )
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
              pkgs.darwin.apple_sdk.frameworks.AppKit
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.darwin.apple_sdk.frameworks.WebKit
            ];
          TAURI_SKIP_DEVSERVER_CHECK = "true";
          # Place the pre-built frontend dist where tauri-build expects it.
          preBuild = ''
            mkdir -p apps/unbill-ui-native/dist
            cp -r ${unbill-ui-dist}/. apps/unbill-ui-native/dist/
          '';
        };

        tauriArtifacts = craneLib.buildDepsOnly (
          tauriArgs
          // {
            cargoExtraArgs = "-p unbill-tauri --features unbill-tauri/custom-protocol";
          }
        );

        unbill-desktop = craneLib.buildPackage (
          tauriArgs
          // {
            cargoArtifacts = tauriArtifacts;
            cargoExtraArgs = "--bin unbill-tauri --features unbill-tauri/custom-protocol";
            doCheck = false;
            meta.mainProgram = "unbill-tauri";
          }
        );
      in
      {
        packages = {
          inherit unbill-cli unbill-tui unbill-desktop;
          default = unbill-cli;
        };
      }
    );
}
