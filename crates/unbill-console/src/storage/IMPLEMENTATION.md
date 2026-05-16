# storage — Implementation

`traits.rs` defines the `LedgerStore` interface. `fs.rs` implements it with flat files and atomic overwrite semantics. `memory.rs` implements it for tests. `http.rs` implements it against a remote HTTPS server.

`FsStore` serializes the `LedgerDoc` to bytes via `doc.save()` and writes `ledger.bin` and `meta.json` under a per-ledger directory. `load_ledger` reads `ledger.bin` and reconstructs the doc via `LedgerDoc::from_bytes`. Writes use `.tmp` sibling + rename for atomicity.

`HttpStore` receives a `&mut LedgerDoc` on each `save_ledger` call and runs the standard Automerge sync loop — the same one used in `net/sync.rs` for Iroh P2P sync — over HTTP instead of a bidirectional stream. Each iteration of the loop is one `POST /ledgers/{id}/sync` request. The server is stateless: it creates a fresh `automerge::sync::State` per request, receives the client's message, generates a response, and saves the document if it changed. No new `LedgerDoc` methods are required; `generate_sync_message` and `receive_sync_message` are reused as-is.

## Dependencies

- `reqwest` (with `json` and `rustls-tls` features, no default features) for the HTTP client
- `wiremock` (dev-dependency) for HTTP mock server in tests
