# Unbill CLI — Implementation

`main.rs` parses commands with `clap`, connects to the running `unbill-daemon` via `RpcAsymChannel` (local socket at `$UNBILL_DATA_DIR/unbill.sock`), creates `UnbillConsole`, and dispatches to `commands.rs`. `output.rs` renders human-readable or JSON output.

`commands.rs` is intentionally thin: it parses command arguments into typed inputs, calls the service, and prints either text or JSON. `output.rs` owns the serializable JSON views so `unbill-core` does not need to derive CLI-facing serialization.

## End-to-end tests

`tests/e2e.rs` runs the real binary against isolated temp directories. Each test gets an `Env` that:

1. Creates a `TempDir` and sets `UNBILL_DATA_DIR` to it.
1. Spawns `unbill-daemon` via `cargo run -p unbill-daemon` with that data directory.
1. Waits for the daemon's `"listening on: <node_id>"` stdout line as a readiness signal; the node ID is stored on `Env` for peer-sync tests.
1. Kills and waits for the daemon on drop.

CLI commands connect to the daemon socket inside the temp directory, so all storage is isolated per test. The suite covers the `--json` output contract, multi-process persistence, and two-env peer sync flows (`join`, `sync once`).
