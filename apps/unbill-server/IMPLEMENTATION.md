# unbill-server — Implementation

The binary is built from `src/main.rs`, which reads environment variables, opens an `FsStore` and an `UnbillDevice`, spawns the `accept_loop` background task (Iroh peer sync), and starts an `axum` HTTP server.

The router lives in `src/router.rs` and is exported by `src/lib.rs` so that integration tests can call it without a real TCP socket.

## Module layout

- `src/main.rs` — reads config, builds the router, binds TCP, runs the server
- `src/lib.rs` — exports `build_router` and `AppState`
- `src/router.rs` — route table, handlers, auth middleware

## Auth middleware

A `tower` middleware function (`auth`) runs before every handler. It extracts the `Authorization` header, checks the `Bearer` scheme, and compares the token to `AppState::api_key`. Mismatches return `401` before reaching any handler.

## Handlers

Each handler receives `State<Arc<AppState>>` (containing the `UnbillDevice` and `api_key`) and calls the corresponding `UnbillDevice` method. Errors map to HTTP responses:

- `UnbillError::Automerge` (malformed client sync message) becomes `400 Bad Request`.
- All other errors become `500 Internal Server Error` with the error message in the body.

## Device key validation

A small helper `valid_device_key(key)` returns `false` if the key contains anything other than `[a-zA-Z0-9._-]`. Handlers return `400` for invalid keys before touching the store.

## SSE event stream

`GET /api/v1/events` returns an `axum::response::sse::Sse` response. The handler calls `state.service.subscribe()` to obtain a `broadcast::Receiver<ServiceEvent>`, then wraps it in a `Stream` that maps each event to an `axum::response::sse::Event` with the JSON-serialised payload as the `data` field. Only `ServiceEvent::LedgerUpdated` is forwarded; other variants are silently skipped. `broadcast::error::RecvError::Lagged` is also skipped — the client will re-poll after reconnection. The stream ends when the receiver returns `RecvError::Closed` (device shut down).

## Dependencies

- `axum 0.8` — routing, extractors, and SSE support (`axum::response::sse`)
- `tower 0.5` — middleware
- `tower-http` — request tracing via `TraceLayer`
- `tokio-stream` — adapts the broadcast channel into a `Stream` for the SSE response
- `clap` is not used; configuration is env-only for simplicity
