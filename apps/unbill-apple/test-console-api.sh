#!/usr/bin/env bash
# Typecheck the generated Console API against each Apple target's real headers.
set -euo pipefail

APP="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$APP/../.." && pwd)"
PROBES="$(mktemp -d)"
trap 'rm -rf "$PROBES"' EXIT
BINDINGS=("$APP"/Sources/Generated/unbill_ffi_*.swift)

if [[ $# -eq 0 ]]; then
  echo "Usage: $0 <rust-target>..." >&2
  exit 2
fi

cat > "$PROBES/InProcess.swift" <<'SWIFT'
func checkInProcess(dir: String) throws -> Console {
    try Console.openInProcess(dir: dir)
}
SWIFT
cat > "$PROBES/Rpc.swift" <<'SWIFT'
func checkRpc(dir: String) throws -> Console {
    try Console.openRpc(dir: dir)
}
SWIFT

for target in "$@"; do
  case "$target" in
    aarch64-apple-ios)
      sdk=iphoneos
      swift_target=arm64-apple-ios17.0
      rpc_supported=false ;;
    aarch64-apple-ios-sim)
      sdk=iphonesimulator
      swift_target=arm64-apple-ios17.0-simulator
      rpc_supported=false ;;
    aarch64-apple-ios-macabi)
      sdk=macosx
      swift_target=arm64-apple-ios17.0-macabi
      rpc_supported=true ;;
    *)
      echo "Unsupported test target: $target" >&2
      exit 2 ;;
  esac
  sdk_root="$("$APP/xcbuild.sh" xcrun --sdk "$sdk" --show-sdk-path)"
  compiler=(
    "$APP/xcbuild.sh" xcrun --sdk "$sdk" swiftc
    -typecheck -target "$swift_target" -sdk "$sdk_root"
    -module-name UnbillConsoleApiCheck
    -module-cache-path "$PROBES/modules/$target"
    -I "$APP/Generated/$target/headers"
    "${BINDINGS[@]}"
  )

  # First prove this SDK, header/module map, and full generated source compile.
  "${compiler[@]}" "$PROBES/InProcess.swift"
  if [[ "$rpc_supported" == true ]]; then
    "${compiler[@]}" "$PROBES/Rpc.swift"
  else
    cargo tree --manifest-path "$ROOT/Cargo.toml" --locked --offline \
      -p unbill-ffi --target "$target" -e normal,build --prefix none \
      > "$PROBES/$target-dependencies.txt"
    if grep -Eq '^(interprocess|tarpc) ' "$PROBES/$target-dependencies.txt"; then
      echo "Desktop RPC dependencies are enabled for $target" >&2
      exit 1
    fi
    if "${compiler[@]}" "$PROBES/Rpc.swift" > "$PROBES/$target.log" 2>&1; then
      echo "RPC construction unexpectedly compiled for $target" >&2
      exit 1
    fi
    if ! grep -Fq "has no member 'openRpc'" "$PROBES/$target.log"; then
      cat "$PROBES/$target.log" >&2
      exit 1
    fi
  fi
  echo "Console API checked: $target (RPC available: $rpc_supported)"
done
