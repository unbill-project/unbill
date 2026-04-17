use autosurgeon::{Hydrate, Reconcile};

use super::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Ledger {
    pub ledger_id: String,
    pub schema_version: u32,
    pub name: String,
    pub currency: String,
    pub created_at: Timestamp,
    pub members: Vec<Member>,
    pub bills: Vec<super::bill::Bill>,
    // Invitations are NOT part of the CRDT. They live in UnbillService memory. See DESIGN.md §6.3.
}

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Member {
    pub user_id: String,
    pub display_name: String,
    pub devices: Vec<Device>,
    pub added_at: Timestamp,
    pub added_by: String,
    pub removed: bool,
}

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Device {
    pub node_id: String,
    pub label: String,
    pub added_at: Timestamp,
}

/// A pending join invitation. Held in `UnbillService` memory only — never
/// persisted or synced. Consumed (removed from the map) on first use or expiry.
#[derive(Clone, Debug)]
pub struct Invitation {
    pub token: String, // random 32 bytes, hex-encoded
    pub ledger_id: String,
    pub created_by_user_id: String,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

/// Lightweight summary for list views (no CRDT bytes needed).
#[derive(Clone, Debug)]
pub struct LedgerMeta {
    pub ledger_id: String,
    pub name: String,
    pub currency: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
