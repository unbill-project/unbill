---
name: Security Model
desc: The modest trust, transport identity, and authorization target for Unbill v1.
category:
  - concept
belongs:
  - unbill
refines:
  - project-boundaries
---

Transport uses Iroh over QUIC and TLS.
Each device identity is a `NodeId` derived from Ed25519 key material.

Authorization is ledger-scoped.
A device may sync a ledger when its `NodeId` is listed in that ledger's device set.

Outbound network traffic is limited to peer discovery, relay fallback,
and direct sync traffic.
The design does not include analytics beacons,
hosted coordination services,
or default update checks.

The current threat model is intentionally modest.
Unbill aims to prevent accidental cross-ledger access and trivial wire impersonation.
It does not try to defend against malicious insiders,
compromised devices,
revocation problems,
or relay metadata leakage.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from): (none)

> **Sirno generated links end.**
