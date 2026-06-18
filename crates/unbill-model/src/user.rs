use autosurgeon::{Hydrate, Reconcile};

use crate::currency::Currency;
use crate::id::{LedgerId, UserId};
use crate::invite_token::InviteToken;
use crate::node_id::NodeId;
use crate::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Ledger {
    // sirno:witness:data-model:begin
    pub ledger_id: LedgerId,
    pub schema_version: u32,
    pub name: String,
    pub currency: Currency,
    pub created_at: Timestamp,
    pub users: Vec<User>,
    pub bills: Vec<crate::bill::Bill>,
    /// Devices authorized to sync this ledger. Any authorized device may record
    /// bills on behalf of any user — there is no per-user device binding.
    pub devices: Vec<Device>,
    // sirno:witness:data-model:end
    // Invitations are NOT part of the CRDT. They live in device-local metadata.
}

// sirno:witness:users-and-devices:begin
#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct User {
    pub user_id: UserId,
    pub display_name: String,
    pub added_at: Timestamp,
}

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Device {
    /// The iroh NodeId (a 32-byte Ed25519 public key).
    pub node_id: NodeId,
    pub added_at: Timestamp,
}

/// Input type for creating a brand-new user (allocates a new UserId).
#[derive(Clone, Debug, garde::Validate)]
pub struct NewUserName {
    #[garde(length(min = 1, max = 100))]
    pub display_name: String,
}

/// Input type for creating a new ledger.
#[derive(Clone, Debug, garde::Validate)]
pub struct NewLedger {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(skip)]
    pub currency: crate::Currency,
}

/// Input type for renaming an existing ledger.
#[derive(Clone, Debug, garde::Validate)]
pub struct NewLedgerName {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
}

/// Input type for directly adding a user to a ledger.
#[derive(Clone, Debug, garde::Validate)]
pub struct NewUser {
    #[garde(skip)]
    pub user_id: UserId,
    #[garde(length(min = 1))]
    pub display_name: String,
}

/// Input type for adding a device to a ledger.
#[derive(Clone, Debug)]
pub struct NewDevice {
    pub node_id: NodeId,
}
// sirno:witness:users-and-devices:end

// sirno:witness:shared-and-local-state:begin
/// A pending join invitation. Stored in device-local metadata, never synced as
/// ledger state. Consumed (removed from the map) on first use or expiry.
///
/// The invitation authorizes a new device (NodeId) to join the ledger. It
/// carries no person record — user management is a separate operation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Invitation {
    pub token: InviteToken,
    pub ledger_id: LedgerId,
    /// The device that issued this invitation.
    pub created_by_device: NodeId,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}
// sirno:witness:shared-and-local-state:end

/// Lightweight summary for list views (no CRDT bytes needed).
#[derive(Clone, Debug)]
pub struct LedgerMeta {
    pub ledger_id: LedgerId,
    pub name: String,
    pub currency: Currency,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
