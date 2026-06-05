---
core.desc: The modest trust, transport identity, and authorization target for Unbill v1.
core.name: Security Model
core.category:
  - core.concept
core.belongs:
  - unbill
core.refines:
  - project-boundaries
---

Transport uses Iroh over QUIC and TLS.
Each device identity is a `NodeId` derived from Ed25519 key material.

Authorization is ledger-scoped.
A device may sync a ledger when its `NodeId` is listed in that ledger's device set.

Outbound network traffic is limited to peer discovery, relay fallback,
and direct sync traffic.
Devices register with Iroh relay servers for NAT traversal
and publish their relay address through pkarr DNS.
Both are third-party services operated by n0.
All traffic through relays is end-to-end encrypted via QUIC/TLS,
so relays can observe connection metadata
(which NodeIDs communicate and when)
but cannot read ledger content.
The design does not include analytics beacons
or default update checks.

The current threat model is intentionally modest.
Unbill aims to prevent accidental cross-ledger access and trivial wire impersonation.
It does not try to defend against malicious insiders,
compromised devices,
revocation problems,
or relay metadata leakage.

---

> **Sirno generated links begin. Do not edit this section.**

- core.belongs (to):
  - [unbill](unbill.md)
- core.belongs (from): (none)

> **Sirno generated links end.**
