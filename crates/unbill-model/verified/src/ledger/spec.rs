// Interface specifications for the verified ledger state machine.
// Defines the state, invariant, and transition predicates.

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

pub type ShareModel = (u64, u32);

pub struct BillModel {
    pub id: u64,
    pub amount_cents: i64,
    pub payers: Seq<ShareModel>,
    pub payees: Seq<ShareModel>,
    pub prev: Seq<u64>,
    pub created_by_device: u64,
}

pub struct UserModel {
    pub user_id: u64,
}

pub struct DeviceModel {
    pub device_id: u64,
}

pub struct LedgerModel {
    pub ledger_id: u64,
    pub users: Seq<UserModel>,
    pub bills: Seq<BillModel>,
    pub devices: Seq<DeviceModel>,
}

// ---------------------------------------------------------------------------
// Helper predicates
// ---------------------------------------------------------------------------

pub open spec fn has_user(users: Seq<UserModel>, user_id: u64) -> bool {
    exists|i: int| 0 <= i < users.len() && #[trigger] users[i].user_id == user_id
}

pub open spec fn has_device(devices: Seq<DeviceModel>, device_id: u64) -> bool {
    exists|i: int| 0 <= i < devices.len() && #[trigger] devices[i].device_id == device_id
}

pub open spec fn has_bill(bills: Seq<BillModel>, bill_id: u64) -> bool {
    exists|i: int| 0 <= i < bills.len() && #[trigger] bills[i].id == bill_id
}

pub open spec fn shares_reference_known_users(shares: Seq<ShareModel>, users: Seq<UserModel>) -> bool {
    forall|i: int| 0 <= i < shares.len() ==>
        has_user(users, #[trigger] shares[i].0)
}

pub open spec fn total_weight(shares: Seq<ShareModel>) -> int
    decreases shares.len(),
{
    if shares.len() == 0 {
        0
    } else {
        total_weight(shares.drop_last()) + shares.last().1 as int
    }
}

pub open spec fn user_ids_unique(users: Seq<UserModel>) -> bool {
    forall|i: int, j: int|
        0 <= i < users.len() && 0 <= j < users.len() && i != j
        ==> #[trigger] users[i].user_id != #[trigger] users[j].user_id
}

pub open spec fn device_ids_unique(devices: Seq<DeviceModel>) -> bool {
    forall|i: int, j: int|
        0 <= i < devices.len() && 0 <= j < devices.len() && i != j
        ==> #[trigger] devices[i].device_id != #[trigger] devices[j].device_id
}

pub open spec fn bill_ids_unique(bills: Seq<BillModel>) -> bool {
    forall|i: int, j: int|
        0 <= i < bills.len() && 0 <= j < bills.len() && i != j
        ==> #[trigger] bills[i].id != #[trigger] bills[j].id
}

pub open spec fn bill_well_formed(
    bill: BillModel, users: Seq<UserModel>, devices: Seq<DeviceModel>, bills: Seq<BillModel>,
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

pub open spec fn state_machine_invariant(ledger: LedgerModel) -> bool {
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
// Transitions
// ---------------------------------------------------------------------------

pub open spec fn init(post: LedgerModel, ledger_id: u64) -> bool {
    &&& post.ledger_id == ledger_id
    &&& post.users.len() == 0
    &&& post.bills.len() == 0
    &&& post.devices.len() == 0
}

pub open spec fn add_user(pre: LedgerModel, post: LedgerModel, user: UserModel) -> bool {
    &&& !has_user(pre.users, user.user_id)
    &&& post.users == pre.users.push(user)
    &&& post.bills == pre.bills
    &&& post.devices == pre.devices
    &&& post.ledger_id == pre.ledger_id
}

pub open spec fn add_device(pre: LedgerModel, post: LedgerModel, device: DeviceModel) -> bool {
    &&& !has_device(pre.devices, device.device_id)
    &&& post.devices == pre.devices.push(device)
    &&& post.bills == pre.bills
    &&& post.users == pre.users
    &&& post.ledger_id == pre.ledger_id
}

pub open spec fn add_bill(pre: LedgerModel, post: LedgerModel, bill: BillModel) -> bool {
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
    &&& post.bills == pre.bills.push(bill)
    &&& post.users == pre.users
    &&& post.devices == pre.devices
    &&& post.ledger_id == pre.ledger_id
}

}
