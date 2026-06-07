// Layer 1: LedgerStateModel — spec types.
// Mirrors the production Ledger/Bill/User/Device/Share structs exactly.
// All IDs are Seq<u8> (matching ULID string representation).

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Spec types (Seq-based, for reasoning)
// ---------------------------------------------------------------------------

/// Mirrors production `Share { user_id: UserId, shares: u32 }`.
pub struct ShareSpec {
    pub user_id: Seq<u8>,
    pub weight: u32,
}

/// Mirrors production `Bill`.
pub struct BillSpec {
    pub id: Seq<u8>,
    pub amount_cents: i64,
    pub description: Seq<u8>,
    pub payers: Seq<ShareSpec>,
    pub payees: Seq<ShareSpec>,
    pub prev: Seq<Seq<u8>>,
    pub created_at: i64,
    pub created_by_device: Seq<u8>,
}

/// Mirrors production `User`.
pub struct UserSpec {
    pub user_id: Seq<u8>,
    pub display_name: Seq<u8>,
    pub added_at: i64,
}

/// Mirrors production `Device`.
pub struct DeviceSpec {
    pub node_id: Seq<u8>,
    pub added_at: i64,
}

/// Mirrors production `Ledger`.
pub struct LedgerStateSpec {
    pub ledger_id: Seq<u8>,
    pub schema_version: u32,
    pub name: Seq<u8>,
    pub currency: Seq<u8>,
    pub created_at: i64,
    pub users: Seq<UserSpec>,
    pub bills: Seq<BillSpec>,
    pub devices: Seq<DeviceSpec>,
}

// ---------------------------------------------------------------------------
// Helper predicates
// ---------------------------------------------------------------------------

pub open spec fn has_user(users: Seq<UserSpec>, user_id: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < users.len() && #[trigger] users[i].user_id == user_id
}

pub open spec fn has_device(devices: Seq<DeviceSpec>, node_id: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < devices.len() && #[trigger] devices[i].node_id == node_id
}

pub open spec fn has_bill(bills: Seq<BillSpec>, bill_id: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < bills.len() && #[trigger] bills[i].id == bill_id
}

pub open spec fn shares_reference_known_users(
    shares: Seq<ShareSpec>, users: Seq<UserSpec>,
) -> bool {
    forall|i: int| 0 <= i < shares.len() ==>
        has_user(users, #[trigger] shares[i].user_id)
}

pub open spec fn total_weight(shares: Seq<ShareSpec>) -> int
    decreases shares.len(),
{
    if shares.len() == 0 {
        0
    } else {
        total_weight(shares.drop_last()) + shares.last().weight as int
    }
}

pub open spec fn user_ids_unique(users: Seq<UserSpec>) -> bool {
    forall|i: int, j: int|
        0 <= i < users.len() && 0 <= j < users.len() && i != j
        ==> #[trigger] users[i].user_id != #[trigger] users[j].user_id
}

pub open spec fn device_ids_unique(devices: Seq<DeviceSpec>) -> bool {
    forall|i: int, j: int|
        0 <= i < devices.len() && 0 <= j < devices.len() && i != j
        ==> #[trigger] devices[i].node_id != #[trigger] devices[j].node_id
}

pub open spec fn bill_ids_unique(bills: Seq<BillSpec>) -> bool {
    forall|i: int, j: int|
        0 <= i < bills.len() && 0 <= j < bills.len() && i != j
        ==> #[trigger] bills[i].id != #[trigger] bills[j].id
}

/// A bill is well-formed within its ledger.
pub open spec fn bill_well_formed(
    bill: BillSpec,
    users: Seq<UserSpec>,
    devices: Seq<DeviceSpec>,
    bills: Seq<BillSpec>,
) -> bool {
    &&& bill.amount_cents >= 0
    &&& bill.payers.len() > 0
    &&& bill.payees.len() > 0
    &&& total_weight(bill.payers) > 0
    &&& total_weight(bill.payees) > 0
    &&& shares_reference_known_users(bill.payers, users)
    &&& shares_reference_known_users(bill.payees, users)
    &&& has_device(devices, bill.created_by_device)
    &&& forall|j: int| 0 <= j < bill.prev.len() ==>
        has_bill(bills, #[trigger] bill.prev[j])
}

// ---------------------------------------------------------------------------
// State machine invariant
// ---------------------------------------------------------------------------

pub open spec fn ledger_invariant(ledger: LedgerStateSpec) -> bool {
    &&& user_ids_unique(ledger.users)
    &&& device_ids_unique(ledger.devices)
    &&& bill_ids_unique(ledger.bills)
    &&& forall|i: int| 0 <= i < ledger.bills.len() ==>
        bill_well_formed(
            #[trigger] ledger.bills[i],
            ledger.users,
            ledger.devices,
            ledger.bills,
        )
}

// ---------------------------------------------------------------------------
// Transitions — one predicate per operation
// ---------------------------------------------------------------------------

/// Initialize a fresh empty ledger.
pub open spec fn init(
    post: LedgerStateSpec,
    ledger_id: Seq<u8>,
    name: Seq<u8>,
    currency: Seq<u8>,
    created_at: i64,
) -> bool {
    &&& post.ledger_id == ledger_id
    &&& post.schema_version == 1
    &&& post.name == name
    &&& post.currency == currency
    &&& post.created_at == created_at
    &&& post.users.len() == 0
    &&& post.bills.len() == 0
    &&& post.devices.len() == 0
}

/// Add a user. Fresh user_id required.
pub open spec fn add_user(
    pre: LedgerStateSpec, post: LedgerStateSpec, user: UserSpec,
) -> bool {
    &&& !has_user(pre.users, user.user_id)
    &&& post == LedgerStateSpec {
        users: pre.users.push(user),
        ..pre
    }
}

/// Add a device. Fresh node_id required.
pub open spec fn add_device(
    pre: LedgerStateSpec, post: LedgerStateSpec, device: DeviceSpec,
) -> bool {
    &&& !has_device(pre.devices, device.node_id)
    &&& post == LedgerStateSpec {
        devices: pre.devices.push(device),
        ..pre
    }
}

/// Add a bill. Well-formed bill with fresh ID required.
pub open spec fn add_bill(
    pre: LedgerStateSpec, post: LedgerStateSpec, bill: BillSpec,
) -> bool {
    &&& bill.amount_cents >= 0
    &&& bill.payers.len() > 0
    &&& bill.payees.len() > 0
    &&& total_weight(bill.payers) > 0
    &&& total_weight(bill.payees) > 0
    &&& shares_reference_known_users(bill.payers, pre.users)
    &&& shares_reference_known_users(bill.payees, pre.users)
    &&& has_device(pre.devices, bill.created_by_device)
    &&& forall|j: int| 0 <= j < bill.prev.len() ==>
        has_bill(pre.bills, #[trigger] bill.prev[j])
    &&& !has_bill(pre.bills, bill.id)
    &&& post == LedgerStateSpec {
        bills: pre.bills.push(bill),
        ..pre
    }
}

}
