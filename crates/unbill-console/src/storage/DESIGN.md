# storage

The storage module is the persistence boundary for unbill. It stores full ledger snapshots, lightweight ledger metadata, and device-local metadata without exposing backend details to higher layers.

## Contract

- `LedgerStore` loads and saves ledger documents as `LedgerDoc`
- ledger metadata supports fast listing without hydrating Automerge bytes
- device-local metadata stores saved users, labels, and pending token state
- callers do not depend on path layout, file names, or server URLs directly
- `save_ledger` takes `&mut LedgerDoc` so the store can apply remote changes back into the document before returning; callers must treat the doc as the authoritative merged state after the call

## Persistence View

```mermaid
flowchart TB
    Store["LedgerStore"]
    subgraph LedgerData["Ledger data"]
        Meta["meta.json / PUT /ledgers/{id}/meta"]
        Snapshot["LedgerDoc / POST /ledgers/{id}/sync"]
    end
    subgraph DeviceData["Device-local data"]
        Key["device_key.bin / PUT /device/{key}"]
        Local["users.json and device labels / PUT /device/{key}"]
        Pending["pending token files / PUT /device/{key}"]
    end

    Store --> LedgerData
    Store --> DeviceData
```

## Implementations

### FsStore

Flat-file store backed by `tokio::fs`. Writes ledger data under
`<root>/ledgers/<id>/` and device metadata as top-level files under `<root>/`.
Writes are atomic via `.tmp` sibling + rename.

### InMemoryStore

In-process store backed by `Mutex<HashMap>`. For unit tests only.

### HttpStore

REST store backed by an HTTPS server. The store receives a live `LedgerDoc`
from the caller, computes only the changes the server has not yet seen, and
applies any changes the server returns back into the same document. A
session-local cache of the last known server heads per ledger is kept so
successive saves send the minimal delta.

#### REST API contract

Authentication: every request carries `Authorization: Bearer <api_key>`.

| Method | Path | Request body | Success | Notes |
|----------|----------------------------|---------------------------|---------|-------------------------------------------|
| `GET` | `/ledgers` | — | 200 JSON array of `LedgerMeta` | Empty array when none exist |
| `PUT` | `/ledgers/{id}/meta` | JSON `LedgerMeta` | 204 | |
| `POST` | `/ledgers/{id}/sync` | JSON `SyncRequest` | 200 JSON `SyncResponse` | Delta exchange; see below |
| `DELETE` | `/ledgers/{id}` | — | 204 | Idempotent |
| `GET` | `/device/{key}` | — | 200 `application/octet-stream` | 404 → `None` |
| `PUT` | `/device/{key}` | `application/octet-stream`| 204 | |

#### Sync endpoint

`POST /ledgers/{id}/sync` is the only endpoint for reading and writing ledger
documents. It replaces separate snapshot GET and PUT endpoints.

```
Request  Content-Type: application/octet-stream  (automerge sync::Message bytes)
Response Content-Type: application/octet-stream  (automerge sync::Message bytes)
         or 204 No Content when the server has no message to send
```

The client runs the same Automerge sync loop used for Iroh P2P sync, but over
HTTP instead of a bidirectional stream:

1. Client creates a fresh `automerge::sync::State`.
1. Client calls `doc.generate_sync_message(state)`. If `None`, the client is
   already up to date and no request is needed.
1. Client POSTs the encoded message to `/ledgers/{id}/sync`.
1. Server loads its document, creates its own fresh `sync::State`, receives the
   message, generates a response message, and saves the document if it changed.
1. Server returns the response message (200) or 204 if it has nothing to send.
1. Client applies the response message and returns to step 2.

The loop terminates when `generate_sync_message` returns `None` (client is
done) and the server returns 204 (server is done). For a passive server
(clients are the only writers), this typically converges in one round trip.

The `LedgerMeta` JSON shape:

```json
{
  "ledger_id": "<ulid>",
  "name": "<string>",
  "currency": "<ISO 4217 code>",
  "created_at_ms": 1234567890,
  "updated_at_ms": 1234567890
}
```

#### Error mapping

- `401` → `StorageError::Unauthorized`
- `404` on device meta → `None` (not an error)
- any other non-2xx → `StorageError::HttpStatus(status, body)`

## Rules

- shared ledger data and local metadata are stored separately
- the store is the only layer that knows how data is laid out on disk or on the server
- local stores (FsStore, InMemoryStore) serialize the doc to bytes; HttpStore exchanges a delta with the server and mutates the doc in place
