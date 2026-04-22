# Unbill CLI — Implementation

`main.rs` parses commands with `clap`, opens `FsStore`, creates `UnbillService`, and dispatches to `commands.rs`. `output.rs` renders human-readable or JSON output.

`commands.rs` is intentionally thin: it parses command arguments into typed inputs, calls the service, and prints either text or JSON. `output.rs` owns the serializable JSON views so `unbill-core` does not need to derive CLI-facing serialization.

End-to-end coverage runs the real binary against temp directories with `UNBILL_DATA_DIR`, so CLI behavior stays coupled to the core service instead of reimplemented in unit tests. The e2e suite also acts as a contract check for the `--json` output shape and the multi-process sync flows.
