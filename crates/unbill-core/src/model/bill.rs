use autosurgeon::{Hydrate, Reconcile};

use super::id::Ulid;
use super::node_id::NodeId;
use super::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Bill {
    pub id: Ulid,
    pub payer_user_id: Ulid,
    pub amount_cents: i64,
    pub description: String,
    /// Who pays how much, expressed as relative share weights.
    /// Equal split among N people = each person gets 1 share.
    /// The users involved in a bill are always derivable from this field; there
    /// is no separate list of bill members.
    pub shares: Vec<Share>,
    /// IDs of bills superseded by this one. Empty for original bills.
    /// A bill whose ID appears in any other bill's `prev` is no longer effective.
    pub prev: Vec<Ulid>,
    pub created_at: Timestamp,
    /// The iroh NodeId of the device that created this bill entry.
    pub created_by_device: NodeId,
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
    /// IDs of bills superseded by this one. Empty for original (non-amendment) bills.
    pub prev: Vec<Ulid>,
}
