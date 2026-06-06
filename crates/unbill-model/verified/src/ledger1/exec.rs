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

}
