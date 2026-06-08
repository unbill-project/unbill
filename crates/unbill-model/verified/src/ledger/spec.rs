// Layer 1: LedgerStateModel — spec types.
// Mirrors the production Ledger/Bill/User/Device/Share structs.
// All IDs are u128 (matching ULID's native 128-bit representation).

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Spec types (Seq-based for collections, u128 for IDs)
// ---------------------------------------------------------------------------

/// Mirrors production `Share { user_id: UserId, shares: u32 }`.
pub struct ShareSpec {
    pub user_id: u128,
    pub weight: u32,
}

/// Mirrors production `Bill`.
pub struct BillSpec {
    pub id: u128,
    pub amount_cents: i64,
    pub description: Seq<u8>,
    pub payers: Seq<ShareSpec>,
    pub payees: Seq<ShareSpec>,
    pub prev: Seq<u128>,
    pub created_at: i64,
    pub created_by_device: Seq<u8>,
}

/// Mirrors production `User`.
pub struct UserSpec {
    pub user_id: u128,
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
    pub ledger_id: u128,
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

pub open spec fn has_user(users: Seq<UserSpec>, user_id: u128) -> bool {
    exists|i: int| 0 <= i < users.len() && #[trigger] users[i].user_id == user_id
}

pub open spec fn has_device(devices: Seq<DeviceSpec>, node_id: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < devices.len() && #[trigger] devices[i].node_id == node_id
}

pub open spec fn has_bill(bills: Seq<BillSpec>, bill_id: u128) -> bool {
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
// Split shares precondition
// ---------------------------------------------------------------------------

/// Precondition for splitting one share list at a given total.
pub open spec fn split_shares_requires(
    shares: Seq<ShareSpec>,
    total_cents: i64,
) -> bool {
    &&& shares.len() > 0
    &&& total_cents >= 0
    &&& total_cents <= i32::MAX as i64
    &&& shares.len() <= i32::MAX as int
    &&& total_weight(shares) > 0
    &&& total_weight(shares) <= i64::MAX as int
}

/// Both payer and payee sides of a bill satisfy split_shares_requires.
pub open spec fn bill_splittable(bill: BillSpec) -> bool {
    &&& split_shares_requires(bill.payers, bill.amount_cents)
    &&& split_shares_requires(bill.payees, bill.amount_cents)
}

// ---------------------------------------------------------------------------
// Effective bill filtering
// ---------------------------------------------------------------------------

/// A bill is superseded if any other bill references it in its prev list.
pub open spec fn is_superseded(bills: Seq<BillSpec>, idx: int) -> bool {
    exists|j: int, k: int|
        0 <= j < bills.len() && j != idx
        && 0 <= k < bills[j].prev.len()
        && #[trigger] bills[j].prev[k] == bills[idx].id
}

/// A bill is effective if it is not superseded.
pub open spec fn is_effective(bills: Seq<BillSpec>, idx: int) -> bool {
    0 <= idx < bills.len() && !is_superseded(bills, idx)
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
    &&& forall|i: int| 0 <= i < ledger.bills.len() ==>
        bill_splittable(#[trigger] ledger.bills[i])
    // Size bounds for overflow safety.
    &&& ledger.users.len() <= i32::MAX as int
    &&& ledger.bills.len() <= i32::MAX as int
    &&& ledger.devices.len() <= i32::MAX as int
}

// ---------------------------------------------------------------------------
// Transitions — one predicate per operation
// ---------------------------------------------------------------------------

/// Initialize a fresh empty ledger.
pub open spec fn init(
    post: LedgerStateSpec,
    ledger_id: u128,
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

/// Add a user. Fresh user_id required; ledger must have room.
pub open spec fn add_user(
    pre: LedgerStateSpec, post: LedgerStateSpec, user: UserSpec,
) -> bool {
    &&& !has_user(pre.users, user.user_id)
    &&& pre.users.len() < i32::MAX as int
    &&& post == LedgerStateSpec {
        users: pre.users.push(user),
        ..pre
    }
}

/// Add a device. Fresh node_id required; ledger must have room.
pub open spec fn add_device(
    pre: LedgerStateSpec, post: LedgerStateSpec, device: DeviceSpec,
) -> bool {
    &&& !has_device(pre.devices, device.node_id)
    &&& pre.devices.len() < i32::MAX as int
    &&& post == LedgerStateSpec {
        devices: pre.devices.push(device),
        ..pre
    }
}

/// Add a bill. Well-formed bill with fresh ID required; ledger must have room.
pub open spec fn add_bill(
    pre: LedgerStateSpec, post: LedgerStateSpec, bill: BillSpec,
) -> bool {
    &&& bill.amount_cents >= 0
    &&& bill.payers.len() > 0
    &&& bill.payees.len() > 0
    &&& total_weight(bill.payers) > 0
    &&& total_weight(bill.payees) > 0
    &&& bill_splittable(bill)
    &&& shares_reference_known_users(bill.payers, pre.users)
    &&& shares_reference_known_users(bill.payees, pre.users)
    &&& has_device(pre.devices, bill.created_by_device)
    &&& forall|j: int| 0 <= j < bill.prev.len() ==>
        has_bill(pre.bills, #[trigger] bill.prev[j])
    &&& !has_bill(pre.bills, bill.id)
    &&& pre.bills.len() < i32::MAX as int
    &&& post == LedgerStateSpec {
        bills: pre.bills.push(bill),
        ..pre
    }
}

// ---------------------------------------------------------------------------
// Exec preconditions
// ---------------------------------------------------------------------------

pub open spec fn exec_add_user_requires(ledger: LedgerStateSpec, user: UserSpec) -> bool {
    &&& ledger_invariant(ledger)
    &&& !has_user(ledger.users, user.user_id)
    &&& ledger.users.len() < i32::MAX as int
}

pub open spec fn exec_add_device_requires(ledger: LedgerStateSpec, device: DeviceSpec) -> bool {
    &&& ledger_invariant(ledger)
    &&& !has_device(ledger.devices, device.node_id)
    &&& ledger.devices.len() < i32::MAX as int
}

pub open spec fn exec_add_bill_requires(ledger: LedgerStateSpec, bill: BillSpec) -> bool {
    &&& ledger_invariant(ledger)
    &&& add_bill(ledger, LedgerStateSpec { bills: ledger.bills.push(bill), ..ledger }, bill)
    &&& ledger.bills.len() < i32::MAX as int
}

}
