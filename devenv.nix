{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  # libc++.so in nixpkgs is a 28-byte linker script; the emulator's dlopen
  # needs a real ELF.  Symlink the versioned ELFs under the bare names.
  # Verus formal verification toolchain (prebuilt release).
  # The verus binary finds its support files (vstd, z3, etc.) via the
  # `verus-root` sentinel relative to the real executable.
  verus = let
    rust-pkgs = pkgs.extend inputs.rust-overlay.overlays.default;
    rust195 = rust-pkgs.rust-bin.stable."1.95.0".default.override {
      extensions = [ "rustc-dev" "llvm-tools" ];
    };
    verus-unwrapped = pkgs.stdenv.mkDerivation {
      pname = "verus";
      version = "0.2026.05.31";
      src = pkgs.fetchzip {
        url = "https://github.com/verus-lang/verus/releases/download/release/0.2026.05.31.5dd6d83/verus-0.2026.05.31.5dd6d83-x86-linux.zip";
        hash = "sha256-/lAnjkRGmI8LF+p4IO03iHBNup8mnUasFGkQpAX7Dz0=";
      };
      nativeBuildInputs = [ pkgs.autoPatchelfHook ];
      buildInputs = [ pkgs.stdenv.cc.cc.lib rust195 ];
      installPhase = ''
        mkdir -p $out/lib/verus
        cp -r . $out/lib/verus/
      '';
    };

    # Shared setup for verus and cargo-verus: rustup shim + Rust 1.95.0 on PATH.
    verus-env = ''
      export VERUS_RUSTUP_SHIM_DIR=$(mktemp -d)
      cat > "$VERUS_RUSTUP_SHIM_DIR/rustup" << 'RUSTUP_SHIM'
      #!/bin/sh
      case "$1" in
        run) shift; shift; [ "$1" = "--" ] && shift; exec "$@" ;;
        install) exit 0 ;;
        *) echo "1.95.0-x86_64-unknown-linux-gnu (verus nix shim)" ;;
      esac
      RUSTUP_SHIM
      chmod +x "$VERUS_RUSTUP_SHIM_DIR/rustup"
      export PATH="$VERUS_RUSTUP_SHIM_DIR:${rust195}/bin:${verus-unwrapped}/lib/verus:$PATH"
    '';

    verus-bin = pkgs.writeShellScriptBin "verus" ''
      ${verus-env}
      exec ${verus-unwrapped}/lib/verus/verus "$@"
    '';

    cargo-verus-bin = pkgs.writeShellScriptBin "cargo-verus" ''
      ${verus-env}
      exec ${verus-unwrapped}/lib/verus/cargo-verus "$@"
    '';

  in pkgs.symlinkJoin {
      name = "verus-tools";
      paths = [ verus-bin cargo-verus-bin ];
    };

  libcxxELF = pkgs.runCommand "libcxx-elf" {} ''
    mkdir -p $out/lib
    ln -s ${pkgs.llvmPackages.libcxx}/lib/libc++.so.1.0    $out/lib/libc++.so
    ln -s ${pkgs.llvmPackages.libcxx}/lib/libc++.so.1.0    $out/lib/libc++.so.1
    ln -s ${pkgs.llvmPackages.libcxx}/lib/libc++abi.so.1.0 $out/lib/libc++abi.so
    ln -s ${pkgs.llvmPackages.libcxx}/lib/libc++abi.so.1.0 $out/lib/libc++abi.so.1
  '';
in
{
  # https://devenv.sh/languages/
  languages.javascript = {
    enable = true;
  };
  android = {
    enable = true;
    platforms.version = [ "32" "34" "36" ];
    systemImageTypes = [ "google_apis_playstore" ];
    abis = [ "arm64-v8a" "x86_64" ];
    cmake.version = [ "3.22.1" ];
    cmdLineTools.version = "11.0";
    tools.version = "26.1.1";
    # platformTools.version defaults to latest from nixpkgs
    buildTools.version = [ "35.0.0" ];
    emulator = {
      enable = true;
    };
    sources.enable = false;
    systemImages.enable = true;
    ndk.enable = true;
    googleAPIs.enable = true;
    googleTVAddOns.enable = true;
    extras = [ "extras;google;gcm" ];
    extraLicenses = [
      "android-sdk-preview-license"
      "android-googletv-license"
      "android-sdk-arm-dbt-license"
      "google-gdk-license"
      "intel-android-extra-license"
      "intel-android-sysimage-license"
      "mips-android-sysimage-license"
    ];
    # android-studio in nixpkgs is x86_64-linux only; on macOS install it
    # separately (Homebrew / JetBrains Toolbox).
    android-studio = lib.mkIf pkgs.stdenv.isLinux {
      enable = true;
      package = pkgs.android-studio;
    };
  };

  languages.rust = {
    enable = true;
    channel = "stable";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
      "rust-std"
    ];
    targets = [
      "wasm32-unknown-unknown"
      "aarch64-linux-android"
      "armv7-linux-androideabi"
      "x86_64-linux-android"
      "i686-linux-android"
    ] ++ lib.optionals pkgs.stdenv.isDarwin [
      # Apple targets for the native SwiftUI frontend (apps/unbill-apple):
      # device, simulator, and Mac Catalyst (macabi).
      "aarch64-apple-ios"
      "aarch64-apple-ios-sim"
      "aarch64-apple-ios-macabi"
    ];
  };

  # https://devenv.sh/packages/
  packages = [
    pkgs.cargo-tauri
    pkgs.cargo-release
    pkgs.trunk
    pkgs.llvmPackages.bintools
    pkgs.prek
    # sirno's read-only-lake unit tests use fs::set_permissions inside a
    # dir they just made read-only, which returns EPERM on macOS (works on
    # Linux). The binary itself is fine, so skip the check phase.
    (inputs.sirno.packages.${pkgs.system}.default.overrideAttrs (_: { doCheck = false; }))
  ] ++ lib.optionals pkgs.stdenv.isLinux [
    # Verus ships a prebuilt x86-linux binary only; skip it on macOS.
    verus
    # GTK/WebKit dependencies only needed on Linux
    # macOS uses native WebKit framework
    pkgs.glib
    pkgs.atkmm
    pkgs.pango
    pkgs.gdk-pixbuf
    pkgs.gtk3
    pkgs.webkitgtk_4_1
  ];

  # Tauri on Linux looks for `studio.sh` to open Android Studio.
  # android-studio isn't packaged for macOS, so only provide this on Linux.
  scripts."studio.sh" = lib.mkIf pkgs.stdenv.isLinux {
    exec = ''
      exec ${pkgs.android-studio}/bin/android-studio "$@"
    '';
  };

  # Shim rustup so the android devenv module doesn't fail — Android Rust
  # targets are already installed via languages.rust.targets above.
  scripts.rustup.exec = ''
    echo "rustup shim (targets managed by Nix): $*"
  '';

  # Android emulator helpers.
  scripts.avd-delete.exec = ''
    avdmanager delete avd --name "unbill_dev"
  '';
  scripts.avd-create.exec = ''
    echo "no" | avdmanager create avd \
      --name "unbill_dev" \
      --package "system-images;android-34;google_apis_playstore;x86_64" \
      --force
  '';
  scripts.avd-start.exec = ''
    LD_LIBRARY_PATH=${libcxxELF}/lib:$LD_LIBRARY_PATH \
      ANDROID_EMULATOR_USE_SYSTEM_LIBS=1 \
      emulator -avd unbill_dev &
    echo "Waiting for emulator to boot..."
    adb wait-for-device shell 'while [[ -z $(getprop sys.boot_completed) ]]; do sleep 1; done'
    echo "Emulator ready."
  '';

  # On NixOS the AGP-bundled aapt2 binary can't run (non-FHS).  Override it
  # with the SDK's aapt2 in the user-level gradle.properties on every shell
  # entry so the path stays current even when the Nix store hash changes.
  enterShell = ''
    # AGP-bundled aapt2 can't run on NixOS (non-FHS). Override it with the
    # SDK's aapt2 so Gradle uses the Nix-provided binary instead.
    mkdir -p ~/.gradle
    grep -v "android.aapt2FromMavenOverride" ~/.gradle/gradle.properties \
      > /tmp/_gp.tmp 2>/dev/null || true
    echo "android.aapt2FromMavenOverride=$ANDROID_HOME/build-tools/35.0.0/aapt2" >> /tmp/_gp.tmp
    mv /tmp/_gp.tmp ~/.gradle/gradle.properties
  '';

  # See full reference at https://devenv.sh/reference/options/
}
