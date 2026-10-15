#!/usr/bin/env bash
# Decode the shared resource and typecheck About on every requested Apple target.
set -euo pipefail
APP="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$APP/../.." && pwd)"
PROBES="$(mktemp -d)"
trap 'rm -rf "$PROBES"' EXIT

"$APP/xcbuild.sh" xcrun swiftc \
  "$APP/Sources/Models/AboutContent.swift" "$APP/Tests/AboutTests.swift" \
  -module-cache-path "$PROBES/modules/host" -o "$PROBES/about-tests"
"$PROBES/about-tests" "$ROOT/crates/unbill-about/content.json"

for target in "$@"; do
  case "$target" in
    aarch64-apple-ios) sdk=iphoneos; swift_target=arm64-apple-ios17.0 ;;
    aarch64-apple-ios-sim) sdk=iphonesimulator; swift_target=arm64-apple-ios17.0-simulator ;;
    aarch64-apple-ios-macabi) sdk=macosx; swift_target=arm64-apple-ios17.0-macabi ;;
    *) echo "Unsupported test target: $target" >&2; exit 2 ;;
  esac
  sdk_root="$("$APP/xcbuild.sh" xcrun --sdk "$sdk" --show-sdk-path)"
  "$APP/xcbuild.sh" xcrun --sdk "$sdk" swiftc \
    -typecheck -target "$swift_target" -sdk "$sdk_root" \
    -module-cache-path "$PROBES/modules/$target" \
    "$APP/Sources/Models/AboutContent.swift" \
    "$APP/Sources/Features/Settings/AboutView.swift"
done
