# unbill-cli — Implementation

`main.rs` parses commands with `clap`, opens `FsStore`, creates `UnbillService`, and dispatches to `commands.rs`. `output.rs` renders human-readable or JSON output.

End-to-end coverage runs the real binary against temp directories with `UNBILL_DATA_DIR`, so CLI behavior stays coupled to the core service instead of reimplemented in unit tests.
