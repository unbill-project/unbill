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

        # Native toolchain — used for CLI, TUI, daemon, and the Tauri Rust backend.
        rustToolchain = pkgs.rust-bin.stable.latest.default;

        # WASM toolchain — used for the Leptos CSR frontend.
        rustToolchainWasm = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        craneLibWasm = (crane.mkLib pkgs).overrideToolchain rustToolchainWasm;


        # ── CLI / TUI / daemon ────────────────────────────────────────────────

        commonArgs = {
          pname = "unbill";
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          # Exclude unbill-tauri here to avoid pulling in GTK/WebKit for these
          # lightweight packages.
          cargoExtraArgs = "-p unbill-cli -p unbill-tui -p unbill-daemon";
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        };

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

        unbill-daemon = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--bin unbill-daemon";
            doCheck = false;
          }
        );

        # ── wasm-bindgen-cli @ 0.2.118 ────────────────────────────────────────
        # The version must match the `wasm-bindgen` entry in Cargo.lock exactly.
        # nixpkgs ships 0.2.108, so we override it to 0.2.118.
        # cargoHash = sha256 of the vendored deps directory (cargo vendor output).

        wasm-bindgen-cli = pkgs.wasm-bindgen-cli.overrideAttrs (old: rec {
          version = "0.2.118";
          src = pkgs.fetchCrate {
            pname = "wasm-bindgen-cli";
            inherit version;
            hash = "sha256-ve783oYH0TGv8Z8lIPdGjItzeLDQLOT5uv/jbFOlZpI=";
          };
          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            inherit src;
            name = "wasm-bindgen-cli-${version}";
            hash = "sha256-EYDfuBlH3zmTxACBL+sjicRna84CvoesKSQVcYiG9P0=";
          };
        });

        # ── Leptos WASM frontend ──────────────────────────────────────────────
        # Step 1: compile the WASM binary with the wasm32 toolchain.

        wasmArgs = {
          pname = "unbill-ui-native-wasm";
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          cargoExtraArgs = "-p unbill-ui-native";
          doCheck = false;
          # cleanCargoSource strips .js files, but api.rs references bridge.js
          # via #[wasm_bindgen(module = "/src/bridge.js")].
          preBuild = ''
            cp ${./apps/unbill-ui-native/src/bridge.js} \
              apps/unbill-ui-native/src/bridge.js
          '';
        };

        wasmArtifacts = craneLibWasm.buildDepsOnly wasmArgs;

        unbillWasmBin = craneLibWasm.buildPackage (
          wasmArgs
          // {
            cargoArtifacts = wasmArtifacts;
            # cargo install doesn't work for WASM targets; copy the .wasm file
            # directly instead.  Cargo uses the package name (hyphens kept).
            installPhaseCommand = ''
              mkdir -p "$out"
              cp target/wasm32-unknown-unknown/release/unbill-ui-native.wasm "$out/"
            '';
          }
        );

        # Step 2: run wasm-bindgen to produce the JS glue + processed .wasm,
        # then assemble the dist directory that the Tauri build expects.
        unbillFrontendDist = pkgs.runCommand "unbill-ui-native-dist" {
          nativeBuildInputs = [ wasm-bindgen-cli ];
        } ''
          mkdir -p "$out"

          wasm-bindgen \
            --target web \
            --out-dir "$out" \
            --out-name unbill_ui_native \
            ${unbillWasmBin}/unbill-ui-native.wasm

          cp ${./apps/unbill-ui-native/style.css} "$out/app.css"
          cp ${./crates/unbill-ui-components/style.css} "$out/components.css"

          cat > "$out/index.html" << 'EOF'
          <!doctype html>
          <html lang="en">
            <head>
              <meta charset="utf-8" />
              <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
              <title>unbill</title>
              <link rel="stylesheet" href="/app.css" />
              <link rel="stylesheet" href="/components.css" />
              <script type="module">
                import init, * as bindings from '/unbill_ui_native.js';
                const wasm = await init({ module_or_path: '/unbill_ui_native_bg.wasm' });
                window.wasmBindings = bindings;
                dispatchEvent(new CustomEvent('TrunkApplicationStarted', { detail: { wasm } }));
              </script>
            </head>
            <body></body>
          </html>
          EOF
        '';

        # ── Tauri desktop app ─────────────────────────────────────────────────
        # tauri-build (the build.rs) needs several non-Rust files at compile
        # time: tauri.conf.json, capabilities/, icons/, and the pre-built WASM
        # dist.  crane's cleanCargoSource strips these, so we copy them back in
        # preBuild from the Nix store.  (gen/ is not committed to git and is
        # produced by tauri-build itself into OUT_DIR.)

        tauriCommonArgs = {
          pname = "unbill-tauri";
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          # --features tauri/custom-protocol switches Tauri from dev mode
          # (which tries to connect to devUrl) to production mode (which
          # serves the embedded frontend assets).  cargo-tauri adds this flag
          # automatically; crane does not.
          cargoExtraArgs = "--package unbill-tauri --features tauri/custom-protocol";
          doCheck = false;

          buildInputs =
            pkgs.lib.optionals pkgs.stdenv.isLinux [
              pkgs.webkitgtk_4_1
              pkgs.gtk3
              pkgs.glib
              pkgs.glib-networking
              pkgs.pango
              pkgs.gdk-pixbuf
              pkgs.libsoup_3
              pkgs.openssl
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
              pkgs.darwin.apple_sdk.frameworks.AppKit
              pkgs.darwin.apple_sdk.frameworks.Carbon
              pkgs.darwin.apple_sdk.frameworks.WebKit
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];

          nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.pkg-config
            pkgs.wrapGAppsHook3
          ];

          # Restore the non-Rust assets that tauri-build reads, and place the
          # pre-built WASM dist where tauri.conf.json's frontendDist points.
          # This hook runs for both buildDepsOnly and buildPackage because
          # build.rs executes in both phases.
          preBuild = ''
            cp ${./crates/unbill-tauri/tauri.conf.json} \
              crates/unbill-tauri/tauri.conf.json

            mkdir -p crates/unbill-tauri/capabilities
            cp -r ${./crates/unbill-tauri/capabilities}/. \
              crates/unbill-tauri/capabilities/

            mkdir -p crates/unbill-tauri/icons
            cp -r ${./crates/unbill-tauri/icons}/. \
              crates/unbill-tauri/icons/

            mkdir -p apps/unbill-ui-native/dist
            cp -r ${unbillFrontendDist}/. \
              apps/unbill-ui-native/dist/
          '';
        };

        tauriArtifacts = craneLib.buildDepsOnly tauriCommonArgs;

        unbill-tauri = craneLib.buildPackage (
          tauriCommonArgs
          // {
            cargoArtifacts = tauriArtifacts;
          }
        );
      in
      {
        packages = {
          inherit unbill-cli unbill-tui unbill-daemon unbill-tauri wasm-bindgen-cli;
          default = unbill-cli;
        };
      }
    );
}
