# net — P2P Networking

The net layer connects unbill devices directly, without any central server. It is responsible for two things: **ongoing document sync** between authorized devices that share a ledger, and the **one-time join handshake** that bootstraps a new device into a ledger it has not seen before.

All network I/O goes through Iroh, which provides QUIC transport with TLS 1.3 and Ed25519 identity. Each device is permanently identified by its `NodeId` — the public half of its Ed25519 key. Iroh handles relay fallback and hole-punching transparently.

## Three protocols

### `unbill/sync/v1` — document sync

Used between devices that both already hold a copy of a ledger. Identified by ALPN token `unbill/sync/v1`.

### `unbill/join/v1` — device join

Used exactly once when a new device joins a ledger for the first time. Identified by ALPN token `unbill/join/v1`.

### `unbill/identity/v1` — identity transfer

Used exactly once when a new device wants to import an existing user identity (user ID and display name) from another device. Identified by ALPN token `unbill/identity/v1`.

## Peer discovery

No separate discovery mechanism is needed. The `devices` list embedded in each ledger's Automerge document contains the `NodeId` of every device authorized to sync that ledger. To sync, a device dials each entry by `NodeId`. Iroh resolves `NodeId` to an IP address using its relay network and peer discovery; the application never handles addresses directly.

## Authorization

A device is authorized to sync a ledger if and only if its `NodeId` appears in `ledger.devices` at the time of connection. The responder reads the ledger document, checks the list, and rejects any initiator not found there. Because Iroh's TLS layer verifies `NodeId` during the handshake, a device cannot claim a `NodeId` it does not own. Devices are append-only; once authorized a device cannot be revoked.

## Sync protocol (`unbill/sync/v1`)

### Framing

Messages are CBOR-encoded structs, each preceded by a 4-byte big-endian length prefix. The entire exchange runs over a single bidirectional Iroh stream per connection.

### Message sequence

```
Initiator                              Responder
  ── Hello ────────────────────────>
  <─ HelloAck ──────────────────────
  ── SyncMsg(ledger=L, ...) ────────>   (Automerge sync messages,
  <─ SyncMsg(ledger=L, ...) ──────────   one per ledger, interleaved)
  ── SyncDone(ledger=L) ────────────>
  <─ SyncDone(ledger=L) ──────────────
  [stream closed when both sides done]
```

**`Hello`** — sent by the initiator immediately after the stream is opened.

```
Hello {
    ledger_ids: Vec<String>,   // ULIDs of ledgers this device holds
}
```

The initiator's `NodeId` is not sent in the message — the responder reads it from the Iroh connection, where it is verified by TLS.

**`HelloAck`** — sent by the responder after authorization checks.

```
HelloAck {
    accepted: Vec<String>,   // ledger IDs where the initiator is authorized
    rejected: Vec<String>,   // ledger IDs not shared or not authorized
}
```

The responder only accepts a ledger if it can load a copy from `LedgerStore` and the TLS-authenticated `NodeId` of the initiator is in `ledger.devices`. Rejected ledgers are dropped silently.

**`SyncMsg`** — carries a single Automerge sync message for one ledger.

```
SyncMsg {
    ledger_id: String,
    payload: Vec<u8>,   // opaque Automerge sync::Message bytes
}
```

Both sides drive the Automerge sync loop independently for each accepted ledger: call `generate_sync_message`, send if non-`None`, call `receive_sync_message` on incoming payloads, repeat. The loop terminates per ledger when a side's `generate_sync_message` returns `None`.

**`SyncDone`** — signals that this side has no more sync messages for a given ledger.

```
SyncDone { ledger_id: String }
```

The stream is closed once both sides have sent `SyncDone` for every accepted ledger.

### Session-local ledger memory

At the start of a session, the accepted ledger documents are loaded from `LedgerStore` into a session-local map. This map exists only for the duration of the connection; nothing is retained after the session closes. Ledger documents that received remote changes are saved back to `LedgerStore` before the session exits.

### SyncState management

Automerge requires a per-(ledger, peer) `SyncState` to track what each peer has already seen. A fresh `SyncState` is created at the start of every connection and discarded when the connection closes. Because sync is always user-initiated and connections are short-lived, there is no persistent SyncState between sessions.

### What triggers sync

After merging incoming changes into the local document, the net layer saves the updated bytes to `LedgerStore` and emits a `LedgerUpdated` event on the service's broadcast channel.

## Join protocol (`unbill/join/v1`)

The join flow is about **device authorization only**. It adds a new `NodeId` to `ledger.devices` so that device can participate in future syncs. It does not add a user. Adding oneself as a named participant (user) is a separate operation performed via `user add` after the device has successfully joined and synced the ledger.

### Invite URL

The inviting device generates a 32-byte cryptographically random `InviteToken`, saves it to `LedgerStore` (keyed by token hex in `pending_invitations.json`), and constructs an invite URL:

```
unbill://join/<ledger_id>/<inviter_node_id_hex>/<token_hex>
```

- `ledger_id`: 26-character ULID of the ledger
- `inviter_node_id_hex`: 64-character hex of the inviting device's `NodeId`
- `token_hex`: 64-character hex of the 32-byte `InviteToken`

The URL is shared out of band (QR code, copy/paste, messaging app). The token is valid until first use or expiry (default: 24 hours).

### Message sequence

```
Requester (new device)                 Host (inviting device)
  ── JoinRequest ──────────────────>
  <─ JoinResponse / JoinError ──────
```

**`JoinRequest`** — sent by the joining device.

```
JoinRequest {
    token: String,       // token_hex from the invite URL
    ledger_id: String,   // which ledger to join (from the invite URL)
    label: String,       // human-readable name for this device (e.g. "Alice's phone")
}
```

The joining device's `NodeId` is not sent in the message — the host reads it from the Iroh connection, where it is verified by TLS. No user identity (user ID, display name) is part of this request.

**`JoinResponse`** — sent by the host on success.

```
JoinResponse {
    ledger_bytes: Vec<u8>,   // full Automerge document snapshot
}
```

**`JoinError`** — sent by the host on failure.

```
JoinError { reason: String }
```

### Host-side join processing

1. Load `pending_invitations.json` from `LedgerStore` and remove the token (consume it). Save the updated map back. Reject with `JoinError` if the token was not found or already consumed.
2. Verify the token has not expired.
3. Verify the `ledger_id` in the request matches the invitation's `ledger_id`.
4. Read the requester's `NodeId` from the TLS-authenticated Iroh connection.
5. Add that `NodeId` (with the provided `label`) to `ledger.devices` in the Automerge document.
6. Save the updated document to `LedgerStore`.
7. Emit `LedgerUpdated` on the service event channel.
8. Send `JoinResponse` with the full document bytes.

### Requester-side join processing

1. Receive `JoinResponse`.
2. Load the ledger document from the received bytes via `LedgerDoc::from_bytes`.
3. Save meta and bytes to `LedgerStore`.
4. Emit `LedgerUpdated`.

The requester is now authorized to sync. To appear as a named participant in bills, a group user must separately add them via `user add` (any authorized device can do this).

## Identity protocol (`unbill/identity/v1`)

A device stores a list of user identities. Each identity is a stable `user_id` (ULID) paired with a `display_name`. A device may hold identities for multiple people (e.g. a shared device) or multiple identities for the same person across different contexts. Identities are stored as device-local metadata alongside the device key.

When setting up a new device, a user can import one of their existing identities from another device rather than creating a fresh one. This protocol transfers a single identity. It does not touch any ledger document.

### Identity invite URL

The existing device generates a 32-byte cryptographically random token associated with a specific `user_id`, saves it to `LedgerStore` (in `pending_identity_tokens.json`), and constructs an invite URL:

```
unbill://identity/<existing_node_id_hex>/<token_hex>
```

The URL is shared out of band. The token is valid until first use or expiry (default: 24 hours). Each token is bound to exactly one identity — a device with multiple identities generates a separate URL per identity.

### Message sequence

```
New device                             Existing device
  ── IdentityRequest ──────────────>
  <─ IdentityResponse / IdentityError
```

**`IdentityRequest`** — sent by the new device.

```
IdentityRequest {
    token: String,   // token_hex from the invite URL
}
```

The new device's `NodeId` is read from the TLS-authenticated Iroh connection by the existing device, but is not used — this protocol does not authorize ledger access.

**`IdentityResponse`** — sent on success.

```
IdentityResponse {
    user_id: String,        // the stable ULID for this user
    display_name: String,   // the user's display name
}
```

**`IdentityError`** — sent on failure.

```
IdentityError { reason: String }
```

### Processing

Existing device: load `pending_identity_tokens.json` from `LedgerStore`, remove the token (consume it), save the updated map back. Reject if the token was not found. Send `IdentityResponse`.

New device: receive `IdentityResponse`, persist `user_id` and `display_name` to device-local storage. The device is now ready to join ledgers.

## Sync modes

Sync is always user-initiated. There is no background polling or automatic triggering.

### `sync once <peer_node_id>`

Dial the specified peer by `NodeId`. Run the full sync exchange for all ledgers shared with that peer. Close the connection when both sides have sent `SyncDone` for every accepted ledger. If the peer is unreachable, exit non-zero with a clear error message.

### `sync daemon`

Open the Iroh endpoint and wait for incoming connections. When a peer dials in, run the full sync exchange for all shared ledgers, then close the connection. The daemon stays running and accepts additional connections until the user stops it. It makes no outbound connections on its own.

`sync daemon` is the counterpart to `sync once`: one device runs the daemon to receive, the other runs `sync once` to initiate. Both end the connection after sync completes.

The daemon exposes `sync status` by returning whether the endpoint is open and the last sync time per ledger.

## Failure modes

| Condition | Behavior |
|-----------|----------|
| Peer unreachable (`sync once`) | Exit non-zero with error message. |
| `NotAuthorized` | Responder sends `HelloAck` with the ledger in `rejected`. Initiator skips it. |
| Token expired or unknown | Host sends `JoinError`. Requester surfaces the message to the user. |
| Token already consumed | Same as expired. |
| Host offline at join time | Connection fails. Known limitation: the inviting device must be online when the invitee joins. |
| Source device offline at identity import | Same: the device holding the identity must be online during `init import`. |
| Malformed message | Connection closed immediately. |
| Iroh relay unreachable | Iroh retries internally. If all transports fail, treated as peer unreachable. |

## Testing

The sync and join protocol logic is tested with in-process channel pairs that stand in for Iroh streams. No real Iroh endpoints are started. Two `LedgerDoc` instances diverge independently, then run the sync message loop over the in-process channels; the test asserts that both docs reach identical state.

The join flow is tested with a mock host that consumes `JoinRequest` messages and returns pre-built `JoinResponse` payloads, verifying that the requester correctly loads and persists the received ledger document.
