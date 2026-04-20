use autosurgeon::{Hydrate, Reconcile};

use super::id::Ulid;
use super::node_id::NodeId;
use super::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Bill {
    pub id: Ulid,
    pub amount_cents: i64,
    pub description: String,
    /// Who paid the bill, expressed as relative share weights of the total amount.
    /// A single payer contributing the full amount = one entry with any positive weight.
    pub payers: Vec<Share>,
    /// Who received the benefit of the bill, expressed as relative share weights.
    /// Equal split among N people = each person gets 1 share.
    /// The users involved in a bill are always derivable from these two fields; there
    /// is no separate list of bill members.
    pub payees: Vec<Share>,
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
    pub amount_cents: i64,
    pub description: String,
    pub payers: Vec<Share>,
    pub payees: Vec<Share>,
    /// IDs of bills superseded by this one. Empty for original (non-amendment) bills.
    pub prev: Vec<Ulid>,
}
