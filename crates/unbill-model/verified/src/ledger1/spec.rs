// Interface specifications for ledger1: multi-ledger state machine with ID generator.

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// An identifier — modeled as a byte sequence (matching ULID string representation).
pub type Id = Seq<u8>;

/// A share: (user_id, weight).
pub type ShareModel = (Id, u32);

pub struct BillModel {
    pub id: Id,
    pub amount_cents: i64,
    pub payers: Seq<ShareModel>,
    pub payees: Seq<ShareModel>,
    pub prev: Seq<Id>,
    pub created_by_device: Id,
}

pub struct UserModel {
    pub user_id: Id,
}

pub struct DeviceModel {
    pub device_id: Id,
}

pub struct LedgerModel {
    pub ledger_id: Id,
    pub users: Seq<UserModel>,
    pub bills: Seq<BillModel>,
    pub devices: Seq<DeviceModel>,
}

/// The global state: multiple ledgers + an ID generator.
pub struct WorldModel {
    pub ledgers: Seq<LedgerModel>,
    /// The set of all IDs ever generated. Every new ID must not be in this set.
    pub generated_ids: Set<Id>,
}

// ---------------------------------------------------------------------------
// Helper predicates
// ---------------------------------------------------------------------------

pub open spec fn has_user(users: Seq<UserModel>, user_id: Id) -> bool {
    exists|i: int| 0 <= i < users.len() && #[trigger] users[i].user_id == user_id
}

pub open spec fn has_device(devices: Seq<DeviceModel>, device_id: Id) -> bool {
    exists|i: int| 0 <= i < devices.len() && #[trigger] devices[i].device_id == device_id
}

pub open spec fn has_bill(bills: Seq<BillModel>, bill_id: Id) -> bool {
    exists|i: int| 0 <= i < bills.len() && #[trigger] bills[i].id == bill_id
}

pub open spec fn has_ledger(ledgers: Seq<LedgerModel>, ledger_id: Id) -> bool {
    exists|i: int| 0 <= i < ledgers.len() && #[trigger] ledgers[i].ledger_id == ledger_id
}

pub open spec fn find_ledger(ledgers: Seq<LedgerModel>, ledger_id: Id) -> int
    recommends has_ledger(ledgers, ledger_id),
{
    choose|i: int| 0 <= i < ledgers.len() && ledgers[i].ledger_id == ledger_id
}

pub open spec fn shares_reference_known_users(
    shares: Seq<ShareModel>, users: Seq<UserModel>,
) -> bool {
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

pub open spec fn ledger_ids_unique(ledgers: Seq<LedgerModel>) -> bool {
    forall|i: int, j: int|
        0 <= i < ledgers.len() && 0 <= j < ledgers.len() && i != j
        ==> #[trigger] ledgers[i].ledger_id != #[trigger] ledgers[j].ledger_id
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

pub open spec fn ledger_well_formed(ledger: LedgerModel) -> bool {
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

/// All IDs in a ledger are tracked by the ID generator.
pub open spec fn ledger_ids_in_generated(ledger: LedgerModel, generated: Set<Id>) -> bool {
    &&& generated.contains(ledger.ledger_id)
    &&& forall|i: int| 0 <= i < ledger.users.len() ==>
        generated.contains(#[trigger] ledger.users[i].user_id)
    &&& forall|i: int| 0 <= i < ledger.devices.len() ==>
        generated.contains(#[trigger] ledger.devices[i].device_id)
    &&& forall|i: int| 0 <= i < ledger.bills.len() ==>
        generated.contains(#[trigger] ledger.bills[i].id)
}

// ---------------------------------------------------------------------------
// State machine invariant
// ---------------------------------------------------------------------------

pub open spec fn state_machine_invariant(world: WorldModel) -> bool {
    &&& ledger_ids_unique(world.ledgers)
    &&& forall|i: int| 0 <= i < world.ledgers.len() ==>
        ledger_well_formed(#[trigger] world.ledgers[i])
    &&& forall|i: int| 0 <= i < world.ledgers.len() ==>
        ledger_ids_in_generated(#[trigger] world.ledgers[i], world.generated_ids)
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

pub open spec fn create_ledger(pre: WorldModel, post: WorldModel, ledger_id: Id) -> bool {
    &&& !pre.generated_ids.contains(ledger_id)
    &&& post.ledgers == pre.ledgers.push(
        LedgerModel {
            ledger_id,
            users: Seq::empty(),
            bills: Seq::empty(),
            devices: Seq::empty(),
        },
    )
    &&& post.generated_ids == pre.generated_ids.insert(ledger_id)
}

pub open spec fn add_user(
    pre: WorldModel, post: WorldModel, ledger_id: Id, user: UserModel,
) -> bool {
    &&& has_ledger(pre.ledgers, ledger_id)
    &&& !pre.generated_ids.contains(user.user_id)
    &&& {
        let idx = find_ledger(pre.ledgers, ledger_id);
        let ledger = pre.ledgers[idx];
        &&& !has_user(ledger.users, user.user_id)
        &&& post.ledgers == pre.ledgers.update(idx, LedgerModel {
            users: ledger.users.push(user),
            ..ledger
        })
    }
    &&& post.generated_ids == pre.generated_ids.insert(user.user_id)
}

pub open spec fn add_device(
    pre: WorldModel, post: WorldModel, ledger_id: Id, device: DeviceModel,
) -> bool {
    &&& has_ledger(pre.ledgers, ledger_id)
    &&& !pre.generated_ids.contains(device.device_id)
    &&& {
        let idx = find_ledger(pre.ledgers, ledger_id);
        let ledger = pre.ledgers[idx];
        &&& !has_device(ledger.devices, device.device_id)
        &&& post.ledgers == pre.ledgers.update(idx, LedgerModel {
            devices: ledger.devices.push(device),
            ..ledger
        })
    }
    &&& post.generated_ids == pre.generated_ids.insert(device.device_id)
}

pub open spec fn add_bill(
    pre: WorldModel, post: WorldModel, ledger_id: Id, bill: BillModel,
) -> bool {
    &&& has_ledger(pre.ledgers, ledger_id)
    &&& !pre.generated_ids.contains(bill.id)
    &&& {
        let idx = find_ledger(pre.ledgers, ledger_id);
        let ledger = pre.ledgers[idx];
        &&& bill.amount_cents >= 0
        &&& bill.payers.len() > 0
        &&& bill.payees.len() > 0
        &&& total_weight(bill.payers) > 0
        &&& total_weight(bill.payees) > 0
        &&& shares_reference_known_users(bill.payers, ledger.users)
        &&& shares_reference_known_users(bill.payees, ledger.users)
        &&& has_device(ledger.devices, bill.created_by_device)
        &&& forall|j: int| 0 <= j < bill.prev.len() ==>
            has_bill(ledger.bills, #[trigger] bill.prev[j])
        &&& post.ledgers == pre.ledgers.update(idx, LedgerModel {
            bills: ledger.bills.push(bill),
            ..ledger
        })
    }
    &&& post.generated_ids == pre.generated_ids.insert(bill.id)
}

}
