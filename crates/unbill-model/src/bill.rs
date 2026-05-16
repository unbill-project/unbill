use autosurgeon::{Hydrate, Reconcile};

use crate::id::{BillId, UserId};
use crate::node_id::NodeId;
use crate::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Bill {
    // sirno:witness:data-model:begin
    pub id: BillId,
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
    pub prev: Vec<BillId>,
    pub created_at: Timestamp,
    /// The iroh NodeId of the device that created this bill entry.
    pub created_by_device: NodeId,
    // sirno:witness:data-model:end
}

#[derive(Clone, Debug, Reconcile, Hydrate, garde::Validate)]
pub struct Share {
    #[garde(skip)]
    pub user_id: UserId,
    #[garde(range(min = 1))]
    pub shares: u32,
}

/// Input type for adding a new bill via the service layer.
#[derive(Clone, Debug, garde::Validate)]
pub struct NewBill {
    #[garde(range(min = 1))]
    pub amount_cents: i64,
    #[garde(length(min = 1, max = 1000))]
    pub description: String,
    #[garde(length(min = 1), dive)]
    pub payers: Vec<Share>,
    #[garde(length(min = 1), dive)]
    pub payees: Vec<Share>,
    /// IDs of bills superseded by this one. Empty for original (non-amendment) bills.
    // sirno:witness:bill-amendment:begin
    #[garde(skip)]
    pub prev: Vec<BillId>,
    // sirno:witness:bill-amendment:end
}
