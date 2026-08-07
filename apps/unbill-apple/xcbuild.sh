#!/usr/bin/env bash
# Run Xcode tooling from inside the unbill devenv/nix shell.
#
# The devenv shell repoints DEVELOPER_DIR/SDKROOT at a Nix Apple SDK and injects
# Nix compiler/linker vars (LD, NIX_LDFLAGS, CC, MACOSX_DEPLOYMENT_TARGET, ...).
# Those are correct for the Rust/Tauri builds but break Xcode's clang/ld driver
# (e.g. `ld: -objc_abi_version '-Xlinker' not supported`). This wrapper scrubs
# them and points at the real Xcode.app, then execs whatever you pass it.
#
# Usage:
#   ./xcbuild.sh xcodegen generate
#   ./xcbuild.sh xcodebuild -project unbill.xcodeproj -scheme unbill \
#       -destination 'platform=iOS Simulator,name=iPhone 17' build
set -euo pipefail

export DEVELOPER_DIR="${DEVELOPER_DIR_OVERRIDE:-/Applications/Xcode.app/Contents/Developer}"

# Scrub Nix toolchain injection so Xcode uses its own clang/ld/SDK.
unset SDKROOT MACOSX_DEPLOYMENT_TARGET \
      CC CXX LD OBJCOPY \
      CC_FOR_BUILD CXX_FOR_BUILD LD_FOR_BUILD OBJCOPY_FOR_BUILD \
      LD_LIBRARY_PATH LD_DYLD_PATH CPATH LIBRARY_PATH \
      CFLAGS CPPFLAGS CXXFLAGS LDFLAGS
for v in $(env | grep -oE '^NIX_[A-Za-z0-9_]+' || true); do
  unset "$v" || true
done

exec "$@"
