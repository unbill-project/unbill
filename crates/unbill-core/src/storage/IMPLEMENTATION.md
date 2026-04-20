# storage — Implementation

`traits.rs` defines the `LedgerStore` interface. `fs.rs` implements it with flat files and atomic overwrite semantics. `memory.rs` implements it for tests.

`FsStore` writes `ledger.bin` and `meta.json` under a per-ledger directory and uses top-level device metadata files for keys, saved users, labels, and pending tokens. Reads return empty values for missing optional data so callers can treat first-run state as normal.
