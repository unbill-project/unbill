# Unbill Daemon — Implementation

`main.rs` is the entire implementation:

1. Opens `FsStore` at `$UNBILL_DATA_DIR` (acquiring the exclusive file lock).
1. Opens `LocalAsymChannel` over the store, which initialises `UnbillDevice`.
1. Runs `channel.accept_loop()` and `rpc::serve(channel, socket)` concurrently via `tokio::select!`.

`accept_loop` binds the Iroh endpoint, waits for it to be ready, then prints `listening on: <node_id>` to stdout. `rpc::serve` binds the local socket and dispatches incoming tarpc calls to the channel.

The `tokio::select!` exits as soon as either task returns — a fatal error in either component brings the daemon down cleanly.

## Tracing

Tracing is directed to stderr so that stdout remains reserved for the single `listening on: <node_id>` readiness line.
