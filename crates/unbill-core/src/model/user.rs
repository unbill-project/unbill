use autosurgeon::{Hydrate, Reconcile};

use super::currency::Currency;
use super::id::Ulid;
use super::invite_token::InviteToken;
use super::node_id::NodeId;
use super::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Ledger {
    pub ledger_id: Ulid,
    pub schema_version: u32,
    pub name: String,
    pub currency: Currency,
    pub created_at: Timestamp,
    pub users: Vec<User>,
    pub bills: Vec<super::bill::Bill>,
    /// Devices authorized to sync this ledger. Any authorized device may record
    /// bills on behalf of any user — there is no per-user device binding.
    pub devices: Vec<Device>,
    // Invitations are NOT part of the CRDT. They live in UnbillService memory. See DESIGN.md §6.3.
}

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct User {
    pub user_id: Ulid,
    pub display_name: String,
    pub added_at: Timestamp,
    pub added_by: Ulid,
}

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Device {
    /// The iroh NodeId (a 32-byte Ed25519 public key).
    pub node_id: NodeId,
    pub label: String,
    pub added_at: Timestamp,
}

/// Input type for directly adding a user to a ledger.
#[derive(Clone, Debug)]
pub struct NewUser {
    pub user_id: Ulid,
    pub display_name: String,
    pub added_by: Ulid,
}

/// Input type for adding a device to a ledger.
#[derive(Clone, Debug)]
pub struct NewDevice {
    pub node_id: NodeId,
    pub label: String,
}

/// A pending join invitation. Held in `UnbillService` memory only — never
/// persisted or synced. Consumed (removed from the map) on first use or expiry.
///
/// The invitation authorizes a new device (NodeId) to join the ledger. It
/// carries no user identity — user management is a separate operation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Invitation {
    pub token: InviteToken,
    pub ledger_id: Ulid,
    /// The device that issued this invitation.
    pub created_by_device: NodeId,
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
