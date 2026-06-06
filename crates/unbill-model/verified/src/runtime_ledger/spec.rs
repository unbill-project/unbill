// Interface specifications for runtime ledger operations.
// Connects runtime types to the ledger1 spec model.

use crate::ledger1::spec as model;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Runtime types
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
// View impls
// ---------------------------------------------------------------------------

impl View for Share {
    type V = model::ShareModel;
    open spec fn view(&self) -> model::ShareModel {
        (self.user_id@, self.weight)
    }
}

impl View for User {
    type V = model::UserModel;
    open spec fn view(&self) -> model::UserModel {
        model::UserModel { user_id: self.user_id@ }
    }
}

impl View for Device {
    type V = model::DeviceModel;
    open spec fn view(&self) -> model::DeviceModel {
        model::DeviceModel { device_id: self.device_id@ }
    }
}

impl View for Bill {
    type V = model::BillModel;
    open spec fn view(&self) -> model::BillModel {
        model::BillModel {
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
    type V = model::LedgerModel;
    open spec fn view(&self) -> model::LedgerModel {
        model::LedgerModel {
            ledger_id: self.ledger_id@,
            users: self.users@.map(|_i, u: User| u@),
            bills: self.bills@.map(|_i, b: Bill| b@),
            devices: self.devices@.map(|_i, d: Device| d@),
        }
    }
}

/// Spec: convert generated_ids to a Set.
pub open spec fn generated_ids_set(ids: Seq<Seq<u8>>) -> Set<model::Id> {
    Set::new(|id: model::Id| ids.contains(id))
}

/// Spec: convert runtime World to spec WorldModel.
pub open spec fn world_view(world: &World) -> model::WorldModel {
    model::WorldModel {
        ledgers: world.ledgers@.map(|_i, l: Ledger| l@),
        generated_ids: generated_ids_set(
            world.generated_ids@.map(|_i, v: Vec<u8>| v@),
        ),
    }
}

// ---------------------------------------------------------------------------
// Transition requires/ensures (single predicate each)
// ---------------------------------------------------------------------------

pub open spec fn create_ledger_requires(world: &World, ledger_id: &Vec<u8>) -> bool {
    &&& model::state_machine_invariant(world_view(world))
    &&& !generated_ids_set(world.generated_ids@.map(|_i, v: Vec<u8>| v@)).contains(ledger_id@)
}

pub open spec fn create_ledger_ensures(pre: &World, post: &World, ledger_id: Seq<u8>) -> bool {
    &&& model::state_machine_invariant(world_view(post))
    &&& model::create_ledger(world_view(pre), world_view(post), ledger_id)
}

pub open spec fn add_user_requires(world: &World, ledger_id: &Vec<u8>, user: &User) -> bool {
    &&& model::state_machine_invariant(world_view(world))
    &&& !generated_ids_set(world.generated_ids@.map(|_i, v: Vec<u8>| v@)).contains(user.user_id@)
    &&& model::has_ledger(world.ledgers@.map(|_i, l: Ledger| l@), ledger_id@)
}

pub open spec fn add_user_ensures(pre: &World, post: &World, ledger_id: Seq<u8>, user: &User) -> bool {
    &&& model::state_machine_invariant(world_view(post))
}

pub open spec fn add_device_requires(world: &World, ledger_id: &Vec<u8>, device: &Device) -> bool {
    &&& model::state_machine_invariant(world_view(world))
    &&& !generated_ids_set(world.generated_ids@.map(|_i, v: Vec<u8>| v@)).contains(device.device_id@)
    &&& model::has_ledger(world.ledgers@.map(|_i, l: Ledger| l@), ledger_id@)
}

pub open spec fn add_device_ensures(pre: &World, post: &World, ledger_id: Seq<u8>, device: &Device) -> bool {
    &&& model::state_machine_invariant(world_view(post))
}

pub open spec fn add_bill_requires(world: &World, ledger_id: &Vec<u8>, bill: &Bill) -> bool {
    &&& model::state_machine_invariant(world_view(world))
    &&& !generated_ids_set(world.generated_ids@.map(|_i, v: Vec<u8>| v@)).contains(bill.id@)
    &&& model::has_ledger(world.ledgers@.map(|_i, l: Ledger| l@), ledger_id@)
    &&& bill.amount_cents >= 0
    &&& bill.payers.len() > 0
    &&& bill.payees.len() > 0
}

pub open spec fn add_bill_ensures(pre: &World, post: &World, ledger_id: Seq<u8>, bill: &Bill) -> bool {
    &&& model::state_machine_invariant(world_view(post))
}

}
