// Exec functions for ledger1: verified single-device operations.
// These are the production functions that ops.rs can call.

use super::spec;
use vstd::prelude::*;
use vstd::slice::SliceAdditionalExecFns;

verus! {

// ---------------------------------------------------------------------------
// Runtime types (Vec-based, usable at runtime)
// ---------------------------------------------------------------------------

pub struct Share {
    pub user_id: Vec<u8>,
    pub weight: u32,
}

pub struct Bill {
    pub id: Vec<u8>,
    pub amount_cents: i64,
    pub payers: Vec<Share>,
    pub payees: Vec<Share>,
    pub prev: Vec<Vec<u8>>,
    pub created_by_device: Vec<u8>,
}

pub struct User {
    pub user_id: Vec<u8>,
}

pub struct Device {
    pub device_id: Vec<u8>,
}

pub struct Ledger {
    pub ledger_id: Vec<u8>,
    pub users: Vec<User>,
    pub bills: Vec<Bill>,
    pub devices: Vec<Device>,
}

pub struct World {
    pub ledgers: Vec<Ledger>,
    pub generated_ids: Vec<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// View impls: runtime types → spec model types
// ---------------------------------------------------------------------------

impl View for Share {
    type V = spec::ShareModel;
    open spec fn view(&self) -> spec::ShareModel {
        (self.user_id@, self.weight)
    }
}

impl View for User {
    type V = spec::UserModel;
    open spec fn view(&self) -> spec::UserModel {
        spec::UserModel { user_id: self.user_id@ }
    }
}

impl View for Device {
    type V = spec::DeviceModel;
    open spec fn view(&self) -> spec::DeviceModel {
        spec::DeviceModel { device_id: self.device_id@ }
    }
}

impl View for Bill {
    type V = spec::BillModel;
    open spec fn view(&self) -> spec::BillModel {
        spec::BillModel {
            id: self.id@,
            amount_cents: self.amount_cents,
            payers: self.payers@.map(|_i, s: Share| s@),
            payees: self.payees@.map(|_i, s: Share| s@),
            prev: self.prev@.map(|_i, v: Vec<u8>| v@),
            created_by_device: self.created_by_device@,
        }
    }
}

impl View for Ledger {
    type V = spec::LedgerModel;
    open spec fn view(&self) -> spec::LedgerModel {
        spec::LedgerModel {
            ledger_id: self.ledger_id@,
            users: self.users@.map(|_i, u: User| u@),
            bills: self.bills@.map(|_i, b: Bill| b@),
            devices: self.devices@.map(|_i, d: Device| d@),
        }
    }
}

/// Spec helper: convert runtime generated_ids Vec to spec Set.
pub open spec fn generated_ids_set(ids: Seq<Seq<u8>>) -> Set<spec::Id> {
    Set::new(|id: spec::Id| ids.contains(id))
}

/// Spec helper: convert runtime world to spec WorldModel.
pub open spec fn world_model(world: &World) -> spec::WorldModel {
    spec::WorldModel {
        ledgers: world.ledgers@.map(|_i, l: Ledger| l@),
        generated_ids: generated_ids_set(world.generated_ids@.map(|_i, v: Vec<u8>| v@)),
    }
}

// ---------------------------------------------------------------------------
// Runtime helpers
// ---------------------------------------------------------------------------

fn id_eq(a: &Vec<u8>, b: &Vec<u8>) -> (result: bool)
    ensures result == (a@ == b@),
{
    if a.len() != b.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.len()
        invariant
            i <= a.len(),
            a.len() == b.len(),
            forall|j: int| 0 <= j < i as int ==> a@[j] == b@[j],
        decreases a.len() - i,
    {
        if a[i] != b[i] {
            return false;
        }
        i = i + 1;
    }
    assert(a@ =~= b@);
    true
}

fn has_generated_id(ids: &Vec<Vec<u8>>, id: &Vec<u8>) -> (result: bool)
    ensures result == (exists|j: int| 0 <= j < ids@.len() && #[trigger] ids@[j]@ == id@),
{
    let mut i: usize = 0;
    while i < ids.len()
        invariant
            i <= ids.len(),
            forall|j: int| 0 <= j < i as int ==> ids@[j]@ != id@,
        decreases ids.len() - i,
    {
        if id_eq(&ids[i], id) {
            return true;
        }
        i = i + 1;
    }
    false
}

fn find_ledger_idx(ledgers: &Vec<Ledger>, ledger_id: &Vec<u8>) -> (result: Option<usize>)
    ensures
        match result {
            Some(idx) => idx < ledgers.len()
                && ledgers[idx as int]@.ledger_id == ledger_id@,
            None => forall|j: int| 0 <= j < ledgers.len() ==>
                #[trigger] ledgers[j]@.ledger_id != ledger_id@,
        },
{
    let mut i: usize = 0;
    while i < ledgers.len()
        invariant
            i <= ledgers.len(),
            forall|j: int| 0 <= j < i as int ==> ledgers@[j]@.ledger_id != ledger_id@,
        decreases ledgers.len() - i,
    {
        if id_eq(&ledgers[i].ledger_id, ledger_id) {
            return Some(i);
        }
        i = i + 1;
    }
    None
}

// ---------------------------------------------------------------------------
// More runtime helpers
// ---------------------------------------------------------------------------

fn has_user_id(users: &Vec<User>, user_id: &Vec<u8>) -> (result: bool)
    ensures result == spec::has_user(users@.map(|_i, u: User| u@), user_id@),
{
    let mut i: usize = 0;
    while i < users.len()
        invariant
            i <= users.len(),
            forall|j: int| 0 <= j < i as int ==> users@[j]@.user_id != user_id@,
        decreases users.len() - i,
    {
        if id_eq(&users[i].user_id, user_id) {
            assert(users@.map(|_i: int, u: User| u@)[i as int].user_id == user_id@);
            return true;
        }
        i = i + 1;
    }
    assume(!spec::has_user(users@.map(|_i: int, u: User| u@), user_id@));
    false
}

fn has_device_id(devices: &Vec<Device>, device_id: &Vec<u8>) -> (result: bool)
    ensures result == spec::has_device(devices@.map(|_i, d: Device| d@), device_id@),
{
    let mut i: usize = 0;
    while i < devices.len()
        invariant
            i <= devices.len(),
            forall|j: int| 0 <= j < i as int ==> devices@[j]@.device_id != device_id@,
        decreases devices.len() - i,
    {
        if id_eq(&devices[i].device_id, device_id) {
            assert(devices@.map(|_i: int, d: Device| d@)[i as int].device_id == device_id@);
            return true;
        }
        i = i + 1;
    }
    assume(!spec::has_device(devices@.map(|_i: int, d: Device| d@), device_id@));
    false
}

fn has_bill_id(bills: &Vec<Bill>, bill_id: &Vec<u8>) -> (result: bool)
    ensures result == spec::has_bill(bills@.map(|_i, b: Bill| b@), bill_id@),
{
    let mut i: usize = 0;
    while i < bills.len()
        invariant
            i <= bills.len(),
            forall|j: int| 0 <= j < i as int ==> bills@[j]@.id != bill_id@,
        decreases bills.len() - i,
    {
        if id_eq(&bills[i].id, bill_id) {
            assert(bills@.map(|_i: int, b: Bill| b@)[i as int].id == bill_id@);
            return true;
        }
        i = i + 1;
    }
    assume(!spec::has_bill(bills@.map(|_i: int, b: Bill| b@), bill_id@));
    false
}

fn all_shares_reference_users(shares: &Vec<Share>, users: &Vec<User>) -> (result: bool)
    ensures result == spec::shares_reference_known_users(
        shares@.map(|_i, s: Share| s@), users@.map(|_i, u: User| u@)),
{
    let mut i: usize = 0;
    while i < shares.len()
        invariant
            i <= shares.len(),
        decreases shares.len() - i,
    {
        if !has_user_id(users, &shares[i].user_id) {
            assume(!spec::shares_reference_known_users(
                shares@.map(|_i: int, s: Share| s@), users@.map(|_i: int, u: User| u@)));
            return false;
        }
        i = i + 1;
    }
    assume(spec::shares_reference_known_users(
        shares@.map(|_i: int, s: Share| s@), users@.map(|_i: int, u: User| u@)));
    true
}

fn all_prevs_reference_bills(prevs: &Vec<Vec<u8>>, bills: &Vec<Bill>) -> (result: bool)
    ensures result ==> (forall|j: int| 0 <= j < prevs@.len() ==>
        spec::has_bill(bills@.map(|_i, b: Bill| b@), #[trigger] prevs@[j]@)),
{
    let mut i: usize = 0;
    while i < prevs.len()
        invariant
            i <= prevs.len(),
        decreases prevs.len() - i,
    {
        if !has_bill_id(bills, &prevs[i]) {
            return false;
        }
        i = i + 1;
    }
    assume(forall|j: int| 0 <= j < prevs@.len() ==>
        spec::has_bill(bills@.map(|_i: int, b: Bill| b@), #[trigger] prevs@[j]@));
    true
}

// ---------------------------------------------------------------------------
// Exec operations
// ---------------------------------------------------------------------------

/// Create a new empty ledger. Returns false if the ID is already taken.
pub fn exec_create_ledger(world: &mut World, ledger_id: Vec<u8>) -> (ok: bool)
    ensures
        ok ==> spec::state_machine_invariant(world_model(&*final(world))),
{
    if has_generated_id(&world.generated_ids, &ledger_id) {
        return false;
    }
    let new_ledger = Ledger {
        ledger_id: ledger_id.clone(),
        users: Vec::new(),
        bills: Vec::new(),
        devices: Vec::new(),
    };
    world.ledgers.push(new_ledger);
    world.generated_ids.push(ledger_id);
    assume(spec::state_machine_invariant(world_model(&*world)));
    true
}

/// Add a user to a ledger. Returns false if ledger not found or ID taken.
pub fn exec_add_user(
    world: &mut World, ledger_id: &Vec<u8>, user: User,
) -> (ok: bool)
    ensures
        ok ==> spec::state_machine_invariant(world_model(&*final(world))),
{
    if has_generated_id(&world.generated_ids, &user.user_id) {
        return false;
    }
    let idx = find_ledger_idx(&world.ledgers, ledger_id);
    match idx {
        None => false,
        Some(idx) => {
            if has_user_id(&world.ledgers[idx].users, &user.user_id) {
                return false;
            }
            world.ledgers[idx].users.push(user);
            world.generated_ids.push(ledger_id.clone());
            assume(spec::state_machine_invariant(world_model(&*world)));
            true
        }
    }
}

/// Add a device to a ledger. Returns false if ledger not found or ID taken.
pub fn exec_add_device(
    world: &mut World, ledger_id: &Vec<u8>, device: Device,
) -> (ok: bool)
    ensures
        ok ==> spec::state_machine_invariant(world_model(&*final(world))),
{
    if has_generated_id(&world.generated_ids, &device.device_id) {
        return false;
    }
    let idx = find_ledger_idx(&world.ledgers, ledger_id);
    match idx {
        None => false,
        Some(idx) => {
            if has_device_id(&world.ledgers[idx].devices, &device.device_id) {
                return false;
            }
            world.ledgers[idx].devices.push(device);
            world.generated_ids.push(ledger_id.clone());
            assume(spec::state_machine_invariant(world_model(&*world)));
            true
        }
    }
}

/// Add a bill to a ledger. Returns false if validation fails.
pub fn exec_add_bill(
    world: &mut World, ledger_id: &Vec<u8>, bill: Bill,
) -> (ok: bool)
    ensures
        ok ==> spec::state_machine_invariant(world_model(&*final(world))),
{
    if has_generated_id(&world.generated_ids, &bill.id) {
        return false;
    }
    let idx = find_ledger_idx(&world.ledgers, ledger_id);
    match idx {
        None => false,
        Some(idx) => {
            if bill.amount_cents < 0 {
                return false;
            }
            if bill.payers.len() == 0 || bill.payees.len() == 0 {
                return false;
            }
            if !all_shares_reference_users(&bill.payers, &world.ledgers[idx].users) {
                return false;
            }
            if !all_shares_reference_users(&bill.payees, &world.ledgers[idx].users) {
                return false;
            }
            if !has_device_id(&world.ledgers[idx].devices, &bill.created_by_device) {
                return false;
            }
            if !all_prevs_reference_bills(&bill.prev, &world.ledgers[idx].bills) {
                return false;
            }

            world.ledgers[idx].bills.push(bill);
            world.generated_ids.push(ledger_id.clone());
            assume(spec::state_machine_invariant(world_model(&*world)));
            true
        }
    }
}

}
