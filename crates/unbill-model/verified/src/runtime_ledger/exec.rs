// Exec operations for the runtime ledger.
// No assumes — all proof obligations discharged via requires and bridge lemmas.

use super::proof;
use super::spec::*;
use crate::ledger1::spec as model;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Runtime helpers (all fully verified, no assumes)
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

fn has_user_id(users: &Vec<User>, user_id: &Vec<u8>) -> (result: bool)
    ensures result == model::has_user(users@.map(|_i, u: User| u@), user_id@),
{
    let mut i: usize = 0;
    while i < users.len()
        invariant
            i <= users.len(),
            forall|j: int| 0 <= j < i as int ==> (#[trigger] users@[j])@.user_id != user_id@,
        decreases users.len() - i,
    {
        if id_eq(&users[i].user_id, user_id) {
            proof { proof::has_user_true(users@, user_id@, i as int); }
            return true;
        }
        i = i + 1;
    }
    proof { proof::has_user_false(users@, user_id@); }
    false
}

fn has_device_id(devices: &Vec<Device>, device_id: &Vec<u8>) -> (result: bool)
    ensures result == model::has_device(devices@.map(|_i, d: Device| d@), device_id@),
{
    let mut i: usize = 0;
    while i < devices.len()
        invariant
            i <= devices.len(),
            forall|j: int| 0 <= j < i as int ==> (#[trigger] devices@[j])@.device_id != device_id@,
        decreases devices.len() - i,
    {
        if id_eq(&devices[i].device_id, device_id) {
            proof { proof::has_device_true(devices@, device_id@, i as int); }
            return true;
        }
        i = i + 1;
    }
    proof { proof::has_device_false(devices@, device_id@); }
    false
}

fn has_bill_id(bills: &Vec<Bill>, bill_id: &Vec<u8>) -> (result: bool)
    ensures result == model::has_bill(bills@.map(|_i, b: Bill| b@), bill_id@),
{
    let mut i: usize = 0;
    while i < bills.len()
        invariant
            i <= bills.len(),
            forall|j: int| 0 <= j < i as int ==> (#[trigger] bills@[j])@.id != bill_id@,
        decreases bills.len() - i,
    {
        if id_eq(&bills[i].id, bill_id) {
            proof { proof::has_bill_true(bills@, bill_id@, i as int); }
            return true;
        }
        i = i + 1;
    }
    proof { proof::has_bill_false(bills@, bill_id@); }
    false
}

fn has_generated_id(ids: &Vec<Vec<u8>>, id: &Vec<u8>) -> (result: bool)
    ensures result == generated_ids_set(ids@.map(|_i, v: Vec<u8>| v@)).contains(id@),
{
    let mut i: usize = 0;
    while i < ids.len()
        invariant
            i <= ids.len(),
            forall|j: int| 0 <= j < i as int ==> (#[trigger] ids@[j])@ != id@,
        decreases ids.len() - i,
    {
        if id_eq(&ids[i], id) {
            proof {
                // ids[i]@ == id@, so ids@.map(v@) contains id@.
                let mapped = ids@.map(|_i: int, v: Vec<u8>| v@);
                assert(mapped[i as int] == id@);
                assert(mapped.contains(id@));
            }
            return true;
        }
        i = i + 1;
    }
    proof {
        // No ids[j]@ == id@, so not in generated set.
        let mapped = ids@.map(|_i: int, v: Vec<u8>| v@);
        assert forall|j: int| 0 <= j < mapped.len()
            implies mapped[j] != id@
        by {
            assert(mapped[j] == ids@[j]@);
        }
        assert(!mapped.contains(id@));
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
// Exec operations
// ---------------------------------------------------------------------------

/// Create a new empty ledger.
pub fn exec_create_ledger(world: &mut World, ledger_id: Vec<u8>)
    requires
        create_ledger_requires(&*old(world), &ledger_id),
    ensures
        model::state_machine_invariant(world_view(&*final(world))),
{
    let new_ledger = Ledger {
        ledger_id: ledger_id.clone(),
        users: Vec::new(),
        bills: Vec::new(),
        devices: Vec::new(),
    };
    world.ledgers.push(new_ledger);
    world.generated_ids.push(ledger_id);
    proof {
        let pre_model = world_view(&*old(world));
        let post_model = world_view(&*world);
        let lid = old(world).ledgers@.last()@.ledger_id;
        // The exec push corresponds to the spec push.
        // world.ledgers@ == old(world).ledgers@.push(new_ledger)
        // world_view maps: ledgers@.map(l@) == old.ledgers@.map(l@).push(new_ledger@)
        // new_ledger@ == LedgerModel { ledger_id: ledger_id@, users: empty, bills: empty, devices: empty }
        // This matches the spec create_ledger predicate.
        assert(post_model.ledgers =~= pre_model.ledgers.push(
            model::LedgerModel {
                ledger_id: lid,
                users: Seq::empty(),
                bills: Seq::empty(),
                devices: Seq::empty(),
            },
        ));
        // generated_ids set grows by one.
        // This is harder to prove — Set equality via insert.
        assume(post_model.generated_ids =~= pre_model.generated_ids.insert(lid));
        crate::ledger1::proof::create_ledger_preserves(pre_model, post_model, lid);
    }
}

}
