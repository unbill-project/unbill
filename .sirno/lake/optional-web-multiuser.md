---
core.name: Optional Web Multiuser
core.desc: Profile-scoped web access and composite ledger device identity on a shared Iroh server.
core.category:
  - core.concept
core.belongs:
  - unbill
core.refines:
  - users-and-devices
---

The web server may run in single-user or multiuser mode.
Single-user mode keeps the current one-key, one-device behavior and its existing
Iroh identity. Multiuser mode lets several authenticated console profiles use
one server process and one Iroh endpoint without sharing their ledger views.
The `DeviceId` and `ProfileId` types are defined in the model; ledger fields,
server profile access, and console binding remain target design.

## Ledger identity

The ledger's device identity becomes a two-branch type:

```rust
enum DeviceId {
    Iroh(NodeId),
    ServerProfile { node_id: NodeId, profile_id: ProfileId },
}

struct ProfileId(Ulid);
```

`Iroh` is the current device identity and remains available to native devices
and the single-user web server. `ServerProfile` is one logical device identity
for a particular profile on a multiuser server. Its `node_id` is the server's
original Iroh device ID: Iroh uses it to address and authenticate the server.
Its `profile_id` is a distinct typed ULID: the server uses it to select a
profile. Several `ServerProfile` values may have the same `node_id`, but each
pair is distinct. A `ProfileId` is stable for a profile and is not a ledger
`UserId`, an Iroh key, or an authentication secret.

Ledger device records use `DeviceId` in place of `node_id`. Bill
`created_by_device` uses the same type, so bills created by two profiles on
one server have distinct attribution. Device labels and membership remain
ledger-scoped. Ledger accounting users remain independent of login profiles;
an authorized profile can still record bills on behalf of any ledger user.

## Console connection and ledger scope

A console authenticates to the server before opening its channel. In multiuser
mode, the server resolves the credential to exactly one `ProfileId` and binds
the connection or session to that profile. The server derives the acting
`DeviceId::ServerProfile` from its own `NodeId` and that authenticated profile;
the console cannot select a different profile by supplying a ULID in a request.
The credential and session mechanism are server concerns; the ULID alone never
grants access.

The server persists each profile's stable ID and authentication binding across
restarts. Ledger archive preferences and other console-local settings belong
to the profile, rather than to all consoles using the server. Known users and
device views are projected from the profile's visible ledgers; transient peer
information exposed to a console must be filtered to those ledgers too.

The server lists only ledgers whose device set contains the exact acting
`ServerProfile` pair. It applies the same membership check to ledger metadata,
document reads and writes, invitation creation, archive preferences, peer
sync triggers, and event subscriptions. A guessed ledger ID must not bypass
the check. Ledger update events for other profiles are not delivered to this
console. An authenticated profile may redeem a valid invitation for a ledger
it does not yet belong to; successful join adds its exact `ServerProfile` pair
before exposing the ledger. The console's channel identity is the acting
`DeviceId`, while network peer operations retain access to the underlying
`NodeId` when an Iroh address is needed. When multiuser mode is disabled, the
channel continues to identify itself as `DeviceId::Iroh(server_node_id)` and
retains the current API-key behavior.

In multiuser mode, the browser's current raw Automerge write endpoint cannot
be treated as an authenticated bill-author command: a caller could supply
another `created_by_device` value. Profile-facing mutations must be mediated
or validated by the server so the authenticated profile determines attribution
and access. Iroh peer sync remains a separate device-to-device path. This
does not add cryptographic proof of human authorship against an authorized
peer that can edit the replicated document.

## Iroh routing and document format

An Iroh connection authenticates the `NodeId`, not a profile. For a
`ServerProfile` target, connection setup uses `node_id`; the join or sync
operation also carries the `profile_id` needed to route to the intended
logical profile on that server. The server checks the selected profile's
ledger membership before serving a sync operation, and checks the connecting
peer's exact authorized `DeviceId` against the ledger. A valid invitation is
the exception for a join operation. If multiple profiles on one server belong
to the same ledger, the selector keeps their operations distinct. A bare
`Iroh` member is checked by its existing `NodeId` rule.

Both `DeviceId` branches use an explicit tagged representation so they cannot
collide. The new ledger schema uses that representation for device membership
and bill attribution. Existing plain-`NodeId` ledger documents are outside
this schema; no compatibility reader or migration is required. Switching an
installation to multiuser mode must not silently expose a ledger to every
profile.

## Implementation boundaries

The model and verified ledger representation must carry the tagged `DeviceId`
in device membership and bill attribution, including uniqueness and
author-reference checks. Console and asymmetric-channel identity must return
the acting `DeviceId`; Iroh connection APIs still use `NodeId`. The HTTP
channel must authenticate and receive its server-assigned profile identity
before loading ledgers. The server must apply profile scope to every relevant
HTTP route and event stream, store stable profile records, and prevent raw
browser sync from bypassing access or attribution checks. The Iroh join and
sync protocol must route composite identities to the selected profile. The
remote UI may display the profile and shared server identity separately.
