# unbill-store-fs

Flat-file `LedgerStore` backed by `tokio::fs`. The default store for desktop and server deployments.

## Layout

```
<root>/
  ledgers/
    <ledger_id>/
      meta.json       — LedgerMeta as JSON
      ledger.bin      — Automerge snapshot bytes
  device_key.bin      — 32-byte raw Ed25519 secret key
  device_labels.json  — map of NodeId → label
  pending_invitations.json
  unbill.lock         — advisory lock file; held open for the lifetime of the FsStore
```

## Rules

- all writes are atomic: data is written to a `.tmp` file then renamed into place
- `list_ledgers` skips directories that are missing `meta.json` or contain invalid data, logging a warning for each
- `delete_ledger` is idempotent; deleting a non-existent ledger is a no-op
- `create_secret_key` is idempotent; it does not overwrite an existing key
- the root directory is created on demand; it need not exist before the first write
