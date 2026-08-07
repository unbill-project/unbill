#!/usr/bin/env bash
# Build the Rust core (unbill-ffi) into UnbillCore.xcframework + Swift bindings
# for the SwiftUI app. Run from inside the devenv shell (needs the Apple Rust
# targets added in devenv.nix). Spike scope: iOS Simulator + Mac Catalyst.
#
#   devenv shell -- ./apps/unbill-apple/build-rust.sh
#
# Outputs (all git-ignored — regenerate with this script):
#   Generated/UnbillCore.xcframework   static libs + headers, per slice
#   Sources/Generated/unbill_ffi.swift UniFFI-generated Swift bindings (app source)
set -euo pipefail

APP="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$APP/../.." && pwd)"
OUT="$APP/Generated"
SWIFT_OUT="$APP/Sources/Generated"
LIB="libunbill_ffi.a"
PROFILE="release"

# Real device (iphoneos), Apple-Silicon simulator, and Mac Catalyst.
TARGETS=(aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-ios-macabi)

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
rm -rf "$OUT"; mkdir -p "$OUT/headers" "$SWIFT_OUT"
cargo run -q -p unbill-ffi --bin uniffi-bindgen -- generate \
  --library "$ROOT/target/${TARGETS[0]}/$PROFILE/$LIB" \
  --language swift --out-dir "$OUT"

# UniFFI emits <name>.swift + <name>FFI.h + <name>FFI.modulemap. The .swift is
# compiled as an app source; the header + modulemap go into the xcframework.
mv "$OUT"/unbill_ffi.swift "$SWIFT_OUT/unbill_ffi.swift"
mv "$OUT"/*FFI.h "$OUT/headers/"
mv "$OUT"/*FFI.modulemap "$OUT/headers/module.modulemap"

echo "==> Assembling UnbillCore.xcframework"
export DEVELOPER_DIR="${DEVELOPER_DIR_OVERRIDE:-/Applications/Xcode.app/Contents/Developer}"
ARGS=()
for t in "${TARGETS[@]}"; do
  ARGS+=(-library "$ROOT/target/$t/$PROFILE/$LIB" -headers "$OUT/headers")
done
rm -rf "$OUT/UnbillCore.xcframework"
xcodebuild -create-xcframework "${ARGS[@]}" -output "$OUT/UnbillCore.xcframework"

echo "==> Done."
echo "    $OUT/UnbillCore.xcframework"
echo "    $SWIFT_OUT/unbill_ffi.swift"
