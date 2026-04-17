# unbill-cli — Implementation Notes

## Dependencies

| Crate | Why |
|-------|-----|
| `unbill-core` | All business logic |
| `clap` | Argument parsing and subcommand dispatch |
| `tokio` | Async runtime for service calls |
| `tracing-subscriber` | Log output formatting |
| `anyhow` | Error propagation |

## Testing strategy

Rust integration tests in `tests/e2e.rs`. Each test creates a `tempfile::TempDir`, sets `UNBILL_DATA_DIR` to point at it, and drives the real binary via `std::process::Command`. Assertions use `--json` output parsed with `serde_json`. No unit tests in the CLI itself — logic lives in `unbill-core`.
