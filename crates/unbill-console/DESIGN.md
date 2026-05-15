# Unbill Core

The core crate defines the shared ledger semantics, settlement rules, and conflict detection. Every frontend depends on it; none reimplement its domain logic.

## Structure

```mermaid
flowchart TB
    Service["service/"]
    Settlement["settlement/"]
    Conflict["conflict/"]
    Storage["unbill-storage"]
    Network["unbill-network"]
    Model["unbill-model"]
    Event["unbill-event"]

    Service --> Settlement
    Service --> Conflict
    Service --> Storage
    Service --> Network
    Service --> Model
    Service --> Event
    Settlement --> Model
    Conflict --> Model
```

`service/` is the public entry point. `settlement/` and `conflict/` are pure logic modules. Storage, networking, domain types, and events live in dedicated crates that this crate composes.

## Surface

`UnbillService` is the main entry point. It manages local users, ledgers, users inside a ledger, bills, invitations, sync, settlement, conflict detection, and service events.

## Invariants

- IDs are typed newtypes and stay opaque outside the core.
- Ledger currency is fixed at creation.
- Bills are append-only; amendment creates a new bill whose `prev` supersedes earlier bills.
- Bill participants must already exist in the ledger.
- Devices authorize sync per ledger and are not bound to specific users.
- A device enters `ledger.devices` in exactly two ways: the creating device is added automatically when the ledger is created; every subsequent device is added by the host during the join protocol. There is no removal.
- `add_device` is idempotent: adding the same `NodeId` twice is a no-op.
- Device labels, pending tokens, and saved users are local metadata rather than shared ledger state.

## Boundaries

- no CLI parsing, Tauri wiring, or UI state
- storage and transport are abstracted behind the `LedgerStore` trait and `unbill-network`
- ledger semantics, projection, and settlement stay in this crate
