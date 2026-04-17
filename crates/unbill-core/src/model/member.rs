use autosurgeon::{Hydrate, Reconcile};

use super::currency::Currency;
use super::id::Ulid;
use super::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Ledger {
    pub ledger_id: Ulid,
    pub schema_version: u32,
    pub name: String,
    pub currency: Currency,
    pub created_at: Timestamp,
    pub members: Vec<Member>,
    pub bills: Vec<super::bill::Bill>,
    // Invitations are NOT part of the CRDT. They live in UnbillService memory. See DESIGN.md §6.3.
}

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Member {
    pub user_id: Ulid,
    pub display_name: String,
    pub devices: Vec<Device>,
    pub added_at: Timestamp,
    pub added_by: Ulid,
    pub removed: bool,
}

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Device {
    /// The iroh NodeId (a 32-byte Ed25519 public key encoded as a string).
    /// This is NOT a ULID — it is assigned by the iroh networking layer.
    pub node_id: String,
    pub label: String,
    pub added_at: Timestamp,
}

/// A pending join invitation. Held in `UnbillService` memory only — never
/// persisted or synced. Consumed (removed from the map) on first use or expiry.
#[derive(Clone, Debug)]
pub struct Invitation {
    pub token: String, // random 32 bytes, hex-encoded — NOT a ULID
    pub ledger_id: Ulid,
    pub created_by_user_id: Ulid,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

/// Lightweight summary for list views (no CRDT bytes needed).
#[derive(Clone, Debug)]
pub struct LedgerMeta {
    pub ledger_id: Ulid,
    pub name: String,
    pub currency: Currency,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
