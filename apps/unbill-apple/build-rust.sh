#!/usr/bin/env bash
# Build the Rust core (unbill-ffi) into UnbillCore.xcframework + Swift bindings
# for the SwiftUI app. Run from inside the devenv shell (needs the Apple Rust
# targets added in devenv.nix). Supports iOS device, Simulator, and Mac Catalyst.
#
#   devenv shell -- ./apps/unbill-apple/build-rust.sh
#   ./apps/unbill-apple/build-rust.sh --catalyst-only
#   ./apps/unbill-apple/build-rust.sh --ios-only
#
# Outputs (all git-ignored — regenerate with this script):
#   Generated/UnbillCore.xcframework   static libs + headers, per slice
#   Sources/Generated/unbill_ffi_*.swift per-target Swift bindings (app source)
set -euo pipefail

APP="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$APP/../.." && pwd)"
OUT="$APP/Generated"
SWIFT_OUT="$APP/Sources/Generated"
LIB="libunbill_ffi.a"
PROFILE="release"

# Real device (iphoneos), Apple-Silicon simulator, and Mac Catalyst.
TARGETS=(aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-ios-macabi)
if [[ $# -eq 1 && "$1" == "--catalyst-only" ]]; then
  TARGETS=(aarch64-apple-ios-macabi)
elif [[ $# -eq 1 && "$1" == "--ios-only" ]]; then
  TARGETS=(aarch64-apple-ios)
elif [[ $# -ne 0 ]]; then
  echo "Usage: $0 [--catalyst-only | --ios-only]" >&2
  exit 2
fi

# The core pulls C crypto (Iroh) via unbill-device, forcing two toolchains that
# don't mix in one cargo process: Nix cc for HOST proc-macros/build-scripts (they
# need Nix's SDK/libSystem paths), Xcode clang for the iOS TARGET C. Setting
# DEVELOPER_DIR=Xcode breaks the Nix host link; leaving it Nix breaks iOS xcrun.
#
# So build in two phases:
#   A) Pristine Nix env — compile & LINK every host artifact (proc-macros, build
#      scripts, the uniffi-bindgen bin). No Apple env touched.
#   B) Xcode env — build only the iOS `--lib` (staticlib/rlib: no linking left),
#      reusing the host artifacts from A. cc-rs compiles the target C with Xcode's
#      clang + the right per-target SDK; DEVELOPER_DIR only affects that.

echo "==> Phase A: host artifacts (Nix toolchain)"
cargo fetch --locked
cargo build -p unbill-ffi --release

echo "==> Phase B: iOS libs (Xcode toolchain) for: ${TARGETS[*]}"
export DEVELOPER_DIR="${DEVELOPER_DIR_OVERRIDE:-/Applications/Xcode.app/Contents/Developer}"
XC_CLANG="$(xcrun -f clang)"; XC_AR="$(xcrun -f ar)"
DEVICE_SDK="$(xcrun --sdk iphoneos --show-sdk-path)"
SIM_SDK="$(xcrun --sdk iphonesimulator --show-sdk-path)"
MACABI_SDK="$(xcrun --sdk macosx --show-sdk-path)"
export CC_aarch64_apple_ios="$XC_CLANG"         AR_aarch64_apple_ios="$XC_AR"
export CFLAGS_aarch64_apple_ios="-isysroot $DEVICE_SDK"
export CC_aarch64_apple_ios_sim="$XC_CLANG"     AR_aarch64_apple_ios_sim="$XC_AR"
export CFLAGS_aarch64_apple_ios_sim="-isysroot $SIM_SDK"
export CC_aarch64_apple_ios_macabi="$XC_CLANG"  AR_aarch64_apple_ios_macabi="$XC_AR"
export CFLAGS_aarch64_apple_ios_macabi="-isysroot $MACABI_SDK"
# rustc's own linker for these targets must be Xcode's clang too — some deps
# (e.g. iroh-relay) are crate-type=cdylib and DO link. The Nix cc-wrapper is
# pinned to arm64-apple-darwin and rejects the cross -target flag.
export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$XC_CLANG"
export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER="$XC_CLANG"
export CARGO_TARGET_AARCH64_APPLE_IOS_MACABI_LINKER="$XC_CLANG"
for t in "${TARGETS[@]}"; do
  # Phase B links no host artifacts (Phase A did), so a per-target SDKROOT is
  # safe here and makes rustc pass -isysroot to the Xcode linker (finds iconv,
  # -lSystem, Security/SystemConfiguration/CoreFoundation in the iOS SDK).
  case "$t" in
    *-sim)    export SDKROOT="$SIM_SDK" ;;
    *-macabi) export SDKROOT="$MACABI_SDK" ;;
    *)        export SDKROOT="$DEVICE_SDK" ;;
  esac
  cargo build -p unbill-ffi --release --target "$t" --lib
done
unset SDKROOT

echo "==> Generating Swift bindings"
rm -rf "$OUT"; mkdir -p "$OUT" "$SWIFT_OUT"
# Remove bindings from an earlier full or single-target build.
rm -f "$SWIFT_OUT"/unbill_ffi*.swift
for t in "${TARGETS[@]}"; do
  target_out="$OUT/$t"
  mkdir -p "$target_out/headers"
  "$ROOT/target/$PROFILE/uniffi-bindgen" generate \
    --library "$ROOT/target/$t/$PROFILE/$LIB" \
    --language swift --out-dir "$target_out"

  # Each generated file includes only this target's exports and API checksums.
  # Exactly one file participates in a Swift build, even in a full XCFramework.
  case "$t" in
    aarch64-apple-ios)
      condition='os(iOS) && !targetEnvironment(macCatalyst) && !targetEnvironment(simulator)' ;;
    aarch64-apple-ios-sim)
      condition='os(iOS) && targetEnvironment(simulator)' ;;
    aarch64-apple-ios-macabi)
      condition='targetEnvironment(macCatalyst)' ;;
  esac
  {
    printf '#if %s\n' "$condition"
    cat "$target_out/unbill_ffi.swift"
    printf '\n#endif\n'
  } > "$SWIFT_OUT/unbill_ffi_$t.swift"
  mv "$target_out"/*FFI.h "$target_out/headers/"
  mv "$target_out"/*FFI.modulemap "$target_out/headers/module.modulemap"
done

echo "==> Assembling UnbillCore.xcframework"
export DEVELOPER_DIR="${DEVELOPER_DIR_OVERRIDE:-/Applications/Xcode.app/Contents/Developer}"
ARGS=()
for t in "${TARGETS[@]}"; do
  ARGS+=(-library "$ROOT/target/$t/$PROFILE/$LIB" -headers "$OUT/$t/headers")
done
rm -rf "$OUT/UnbillCore.xcframework"
xcodebuild -create-xcframework "${ARGS[@]}" -output "$OUT/UnbillCore.xcframework"

echo "==> Checking platform-specific Console APIs"
"$APP/test-console-api.sh" "${TARGETS[@]}"

echo "==> Done."
echo "    $OUT/UnbillCore.xcframework"
echo "    $SWIFT_OUT/unbill_ffi_*.swift"
