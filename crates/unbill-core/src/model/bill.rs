use autosurgeon::{Hydrate, Reconcile};

use super::amendment::Amendment;
use super::id::Ulid;
use super::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Bill {
    pub id: Ulid,
    pub payer_user_id: Ulid,
    pub amount_cents: i64,
    pub description: String,
    /// Who pays how much, expressed as relative share weights.
    /// Equal split among N people = each person gets 1 share.
    /// Participants are always derivable from this field; there is no separate
    /// `participant_user_ids` list.
    pub shares: Vec<Share>,
    pub created_at: Timestamp,
    /// The iroh NodeId string of the device that created this bill.
    pub created_by_device: String,
    pub deleted: bool,
    pub amendments: Vec<Amendment>,
}

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Share {
    pub user_id: Ulid,
    pub shares: u32,
}

/// Input type for adding a new bill via the service layer.
#[derive(Clone, Debug)]
pub struct NewBill {
    pub payer_user_id: Ulid,
    pub amount_cents: i64,
    pub description: String,
    pub shares: Vec<Share>,
}
