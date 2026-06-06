// Proof utilities for runtime ledger operations.
// Bridge lemmas connecting runtime Vec views to spec Seq model predicates.

use super::spec::*;
use crate::ledger1::spec as model;
use vstd::prelude::*;

verus! {

/// Lemma: if no runtime user matches, the spec has_user is false.
pub proof fn has_user_false(users: Seq<User>, user_id: Seq<u8>)
    requires
        forall|j: int| 0 <= j < users.len() ==> (#[trigger] users[j])@.user_id != user_id,
    ensures
        !model::has_user(users.map(|_i, u: User| u@), user_id),
{
    let mapped = users.map(|_i: int, u: User| u@);
    assert forall|k: int| 0 <= k < mapped.len()
        implies (#[trigger] mapped[k]).user_id != user_id
    by {
        assert(mapped[k] == users[k]@);
    }
}

/// Lemma: if runtime user at index k matches, the spec has_user is true.
pub proof fn has_user_true(users: Seq<User>, user_id: Seq<u8>, k: int)
    requires
        0 <= k < users.len(),
        users[k]@.user_id == user_id,
    ensures
        model::has_user(users.map(|_i, u: User| u@), user_id),
{
    assert(users.map(|_i: int, u: User| u@)[k] == users[k]@);
}

/// Lemma: if no runtime device matches, the spec has_device is false.
pub proof fn has_device_false(devices: Seq<Device>, device_id: Seq<u8>)
    requires
        forall|j: int| 0 <= j < devices.len() ==> (#[trigger] devices[j])@.device_id != device_id,
    ensures
        !model::has_device(devices.map(|_i, d: Device| d@), device_id),
{
    let mapped = devices.map(|_i: int, d: Device| d@);
    assert forall|k: int| 0 <= k < mapped.len()
        implies (#[trigger] mapped[k]).device_id != device_id
    by {
        assert(mapped[k] == devices[k]@);
    }
}

/// Lemma: if runtime device at index k matches, the spec has_device is true.
pub proof fn has_device_true(devices: Seq<Device>, device_id: Seq<u8>, k: int)
    requires
        0 <= k < devices.len(),
        devices[k]@.device_id == device_id,
    ensures
        model::has_device(devices.map(|_i, d: Device| d@), device_id),
{
    assert(devices.map(|_i: int, d: Device| d@)[k] == devices[k]@);
}

/// Lemma: if no runtime bill matches, the spec has_bill is false.
pub proof fn has_bill_false(bills: Seq<Bill>, bill_id: Seq<u8>)
    requires
        forall|j: int| 0 <= j < bills.len() ==> (#[trigger] bills[j])@.id != bill_id,
    ensures
        !model::has_bill(bills.map(|_i, b: Bill| b@), bill_id),
{
    let mapped = bills.map(|_i: int, b: Bill| b@);
    assert forall|k: int| 0 <= k < mapped.len()
        implies (#[trigger] mapped[k]).id != bill_id
    by {
        assert(mapped[k] == bills[k]@);
    }
}

/// Lemma: if runtime bill at index k matches, the spec has_bill is true.
pub proof fn has_bill_true(bills: Seq<Bill>, bill_id: Seq<u8>, k: int)
    requires
        0 <= k < bills.len(),
        bills[k]@.id == bill_id,
    ensures
        model::has_bill(bills.map(|_i, b: Bill| b@), bill_id),
{
    assert(bills.map(|_i: int, b: Bill| b@)[k] == bills[k]@);
}

/// Lemma: if no generated id matches, it's not in the set.
pub proof fn generated_id_false(ids: Seq<Seq<u8>>, id: Seq<u8>)
    requires
        forall|j: int| 0 <= j < ids.len() ==> (#[trigger] ids[j]) != id,
    ensures
        !generated_ids_set(ids).contains(id),
{
    assert(!ids.contains(id));
}

/// Lemma: if generated id at index k matches, it's in the set.
pub proof fn generated_id_true(ids: Seq<Seq<u8>>, id: Seq<u8>, k: int)
    requires
        0 <= k < ids.len(),
        ids[k] == id,
    ensures
        generated_ids_set(ids).contains(id),
{
    assert(ids.contains(id));
}

}
