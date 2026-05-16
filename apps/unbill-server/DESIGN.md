# unbill-server

`unbill-server` is a standalone HTTP server that exposes the device API consumed by `HttpAsymChannel`. It is the remote backend for a single device: one device identity, one set of ledgers.

## Purpose

Provides a hosted backend so that a console using `HttpAsymChannel` can reach the device API over a network instead of a local socket. A single running instance serves one authenticated client identified by a static API key.

## API

All endpoints require `Authorization: Bearer <api_key>`. Requests with a missing or wrong key receive `401 Unauthorized`.

| Method | Path | Request body | Success |
|----------|-------------------------------|-------------------------------|----------------------|
| `GET` | `/ledgers` | — | 200 JSON array |
| `PUT` | `/ledgers/{id}/meta` | JSON `LedgerMeta` | 204 |
| `POST` | `/ledgers/{id}/sync` | `application/octet-stream` | 200 bytes / 204 |
| `POST` | `/ledgers/{id}/invitations` | — | 200 JSON `{ url }` |
| `POST` | `/ledgers/join` | JSON `{ url, label? }` | 204 |
| `POST` | `/peers/{node_id}/sync` | — | 204 |
| `GET` | `/device/id` | — | 200 plain text |
| `GET` | `/device/{key}` | — | 200 bytes / 404 |
| `PUT` | `/device/{key}` | `application/octet-stream` | 204 |
| `GET` | `/events` | — | 200 SSE stream |

`POST /ledgers/{id}/sync` exchanges a binary Automerge sync message. The client sends its sync message bytes; the server responds with its own message (200) or nothing (204) when it has nothing new. An unparseable message body returns `400`.

`GET /events` opens a persistent SSE stream. The server subscribes to the device event bus and forwards each event as a JSON-encoded SSE `data` field. The stream never sends a terminal event; the client is expected to reconnect if it drops. Each event is a JSON object with a `type` discriminant, matching the `AsymChannelEvent` schema:

```
data: {"type":"LedgerUpdated","ledger_id":"<id>"}
```

Device key names must consist solely of alphanumeric characters, hyphens, underscores, and dots. Any other key is rejected with `400 Bad Request`.

## Configuration

All configuration is read from environment variables at startup. The server exits immediately if a required variable is absent.

| Variable | Required | Default | Description |
|-----------|----------|---------|----------------------------------------------------|
| `API_KEY` | yes | — | Bearer token clients must supply |
| `PORT` | no | `8080` | TCP port to listen on |

The data directory is resolved by `unbill_store_fs::UNBILL_PATH.ensure_data_dir()`, which uses the `UNBILL_DATA_DIR` environment variable if set, or the platform default (`~/.local/share/unbill` on Linux, `~/Library/Application Support/unbill` on macOS) otherwise.

## Boundaries

- One API key, one device namespace. Multi-tenancy is outside scope.
- TLS termination is expected to happen at a reverse proxy; the server itself speaks plain HTTP.
- The server does not perform ledger-level access control beyond the single API key.
