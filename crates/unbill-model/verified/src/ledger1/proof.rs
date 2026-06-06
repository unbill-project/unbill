// Proof utilities for ledger1: each transition preserves the state machine invariant.

use super::spec::*;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Helper lemma: updating one ledger preserves invariants of other ledgers.
// ---------------------------------------------------------------------------

proof fn other_ledgers_preserved(
    pre_ledgers: Seq<LedgerModel>,
    post_ledgers: Seq<LedgerModel>,
    idx: int,
    new_ledger: LedgerModel,
)
    requires
        0 <= idx < pre_ledgers.len(),
        post_ledgers == pre_ledgers.update(idx, new_ledger),
        forall|i: int| 0 <= i < pre_ledgers.len() ==>
            ledger_well_formed(#[trigger] pre_ledgers[i]),
        ledger_well_formed(new_ledger),
    ensures
        forall|i: int| 0 <= i < post_ledgers.len() ==>
            ledger_well_formed(#[trigger] post_ledgers[i]),
{
    assert forall|i: int| 0 <= i < post_ledgers.len()
        implies ledger_well_formed(#[trigger] post_ledgers[i])
    by {
        if i == idx {
            assert(post_ledgers[i] == new_ledger);
        } else {
            assert(post_ledgers[i] == pre_ledgers[i]);
        }
    }
}

proof fn other_ledger_ids_preserved(
    pre_ledgers: Seq<LedgerModel>,
    post_ledgers: Seq<LedgerModel>,
    idx: int,
    new_ledger: LedgerModel,
    generated: Set<Id>,
)
    requires
        0 <= idx < pre_ledgers.len(),
        post_ledgers == pre_ledgers.update(idx, new_ledger),
        forall|i: int| 0 <= i < pre_ledgers.len() ==>
            ledger_ids_in_generated(#[trigger] pre_ledgers[i], generated),
        ledger_ids_in_generated(new_ledger, generated),
    ensures
        forall|i: int| 0 <= i < post_ledgers.len() ==>
            ledger_ids_in_generated(#[trigger] post_ledgers[i], generated),
{
    assert forall|i: int| 0 <= i < post_ledgers.len()
        implies ledger_ids_in_generated(#[trigger] post_ledgers[i], generated)
    by {
        if i == idx {
            assert(post_ledgers[i] == new_ledger);
        } else {
            assert(post_ledgers[i] == pre_ledgers[i]);
        }
    }
}

// ---------------------------------------------------------------------------
// create_ledger preserves invariant
// ---------------------------------------------------------------------------

pub proof fn create_ledger_preserves(pre: WorldModel, post: WorldModel, ledger_id: Id)
    requires
        state_machine_invariant(pre),
        create_ledger(pre, post, ledger_id),
    ensures
        state_machine_invariant(post),
{
    // Ledger ID uniqueness: new ledger_id is fresh (not in generated_ids,
    // and all existing ledger IDs are in generated_ids).
    assert forall|i: int, j: int|
        0 <= i < post.ledgers.len() && 0 <= j < post.ledgers.len() && i != j
        implies #[trigger] post.ledgers[i].ledger_id != #[trigger] post.ledgers[j].ledger_id
    by {
        if i < pre.ledgers.len() && j < pre.ledgers.len() {
        } else if i == pre.ledgers.len() as int {
            // new ledger vs old: new ID not in generated_ids, old ID is.
            assert(post.ledgers[i].ledger_id == ledger_id);
            assert(!pre.generated_ids.contains(ledger_id));
            assert(ledger_ids_in_generated(pre.ledgers[j], pre.generated_ids));
            assert(pre.generated_ids.contains(pre.ledgers[j].ledger_id));
        } else {
            assert(post.ledgers[j].ledger_id == ledger_id);
            assert(!pre.generated_ids.contains(ledger_id));
            assert(ledger_ids_in_generated(pre.ledgers[i], pre.generated_ids));
            assert(pre.generated_ids.contains(pre.ledgers[i].ledger_id));
        }
    }

    // All ledgers well-formed: old ones unchanged, new one is empty.
    assert forall|i: int| 0 <= i < post.ledgers.len()
        implies ledger_well_formed(#[trigger] post.ledgers[i])
    by {
        if i < pre.ledgers.len() as int {
            assert(post.ledgers[i] == pre.ledgers[i]);
        }
        // New ledger: empty users/bills/devices — trivially well-formed.
    }

    // All IDs tracked: old ledgers' IDs still in (superset) generated_ids.
    // New ledger's ID is inserted.
    assert forall|i: int| 0 <= i < post.ledgers.len()
        implies ledger_ids_in_generated(#[trigger] post.ledgers[i], post.generated_ids)
    by {
        if i < pre.ledgers.len() as int {
            assert(post.ledgers[i] == pre.ledgers[i]);
            assert(ledger_ids_in_generated(pre.ledgers[i], pre.generated_ids));
            // pre.generated_ids ⊂ post.generated_ids
        }
        // New ledger: ledger_id inserted, no users/bills/devices.
    }
}

// ---------------------------------------------------------------------------
// add_user preserves invariant
// ---------------------------------------------------------------------------

pub proof fn add_user_preserves(
    pre: WorldModel, post: WorldModel, ledger_id: Id, user: UserModel,
)
    requires
        state_machine_invariant(pre),
        add_user(pre, post, ledger_id, user),
    ensures
        state_machine_invariant(post),
{
    let idx = find_ledger(pre.ledgers, ledger_id);
    let ledger = pre.ledgers[idx];
    let new_ledger = LedgerModel { users: ledger.users.push(user), ..ledger };

    // New ledger is well-formed: user IDs unique (fresh ID from generator).
    assert forall|i: int, j: int|
        0 <= i < new_ledger.users.len() && 0 <= j < new_ledger.users.len() && i != j
        implies #[trigger] new_ledger.users[i].user_id != #[trigger] new_ledger.users[j].user_id
    by {
        if i < ledger.users.len() && j < ledger.users.len() {
        } else if i == ledger.users.len() as int {
            assert(new_ledger.users[i].user_id == user.user_id);
            assert(!pre.generated_ids.contains(user.user_id));
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
            assert(pre.generated_ids.contains(ledger.users[j].user_id));
        } else {
            assert(new_ledger.users[j].user_id == user.user_id);
            assert(!pre.generated_ids.contains(user.user_id));
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
            assert(pre.generated_ids.contains(ledger.users[i].user_id));
        }
    }

    // Bills still well-formed: users grew, shares still reference known users.
    assert forall|i: int| 0 <= i < new_ledger.bills.len()
        implies bill_well_formed(
            #[trigger] new_ledger.bills[i], new_ledger.users, new_ledger.devices, new_ledger.bills,
        )
    by {
        let bill = new_ledger.bills[i];
        assert forall|k: int| 0 <= k < bill.payers.len()
            implies has_user(new_ledger.users, #[trigger] bill.payers[k].0)
        by {
            assert(has_user(ledger.users, bill.payers[k].0));
            let w = choose|w: int| 0 <= w < ledger.users.len() && ledger.users[w].user_id == bill.payers[k].0;
            assert(new_ledger.users[w].user_id == bill.payers[k].0);
        }
        assert forall|k: int| 0 <= k < bill.payees.len()
            implies has_user(new_ledger.users, #[trigger] bill.payees[k].0)
        by {
            assert(has_user(ledger.users, bill.payees[k].0));
            let w = choose|w: int| 0 <= w < ledger.users.len() && ledger.users[w].user_id == bill.payees[k].0;
            assert(new_ledger.users[w].user_id == bill.payees[k].0);
        }
        assert forall|j: int| 0 <= j < bill.prev.len()
            implies has_bill(new_ledger.bills, #[trigger] bill.prev[j])
        by {
            assert(has_bill(ledger.bills, bill.prev[j]));
            let w = choose|w: int| 0 <= w < ledger.bills.len() && ledger.bills[w].id == bill.prev[j];
            assert(new_ledger.bills[w].id == bill.prev[j]);
        }
    }

    // Ledger IDs unique: only the users field changed, ledger_id unchanged.
    assert forall|i: int, j: int|
        0 <= i < post.ledgers.len() && 0 <= j < post.ledgers.len() && i != j
        implies #[trigger] post.ledgers[i].ledger_id != #[trigger] post.ledgers[j].ledger_id
    by {
        if i == idx {
            assert(post.ledgers[i].ledger_id == ledger.ledger_id);
        }
        if j == idx {
            assert(post.ledgers[j].ledger_id == ledger.ledger_id);
        }
    }

    other_ledgers_preserved(pre.ledgers, post.ledgers, idx, new_ledger);

    // IDs in generated: new user ID inserted, old IDs preserved.
    assert forall|i: int| 0 <= i < post.ledgers.len()
        implies ledger_ids_in_generated(#[trigger] post.ledgers[i], post.generated_ids)
    by {
        if i == idx {
            assert(post.ledgers[i] == new_ledger);
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
        } else {
            assert(post.ledgers[i] == pre.ledgers[i]);
            assert(ledger_ids_in_generated(pre.ledgers[i], pre.generated_ids));
        }
    }
}

// ---------------------------------------------------------------------------
// add_device preserves invariant
// ---------------------------------------------------------------------------

pub proof fn add_device_preserves(
    pre: WorldModel, post: WorldModel, ledger_id: Id, device: DeviceModel,
)
    requires
        state_machine_invariant(pre),
        add_device(pre, post, ledger_id, device),
    ensures
        state_machine_invariant(post),
{
    let idx = find_ledger(pre.ledgers, ledger_id);
    let ledger = pre.ledgers[idx];
    let new_ledger = LedgerModel { devices: ledger.devices.push(device), ..ledger };

    // Device IDs unique.
    assert forall|i: int, j: int|
        0 <= i < new_ledger.devices.len() && 0 <= j < new_ledger.devices.len() && i != j
        implies #[trigger] new_ledger.devices[i].device_id != #[trigger] new_ledger.devices[j].device_id
    by {
        if i < ledger.devices.len() && j < ledger.devices.len() {
        } else if i == ledger.devices.len() as int {
            assert(!pre.generated_ids.contains(device.device_id));
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
            assert(pre.generated_ids.contains(ledger.devices[j].device_id));
        } else {
            assert(!pre.generated_ids.contains(device.device_id));
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
            assert(pre.generated_ids.contains(ledger.devices[i].device_id));
        }
    }

    // Bills still well-formed: devices grew.
    assert forall|i: int| 0 <= i < new_ledger.bills.len()
        implies bill_well_formed(
            #[trigger] new_ledger.bills[i], new_ledger.users, new_ledger.devices, new_ledger.bills,
        )
    by {
        let bill = new_ledger.bills[i];
        assert(has_device(ledger.devices, bill.created_by_device));
        let w = choose|w: int| 0 <= w < ledger.devices.len() && ledger.devices[w].device_id == bill.created_by_device;
        assert(new_ledger.devices[w].device_id == bill.created_by_device);
        assert forall|j: int| 0 <= j < bill.prev.len()
            implies has_bill(new_ledger.bills, #[trigger] bill.prev[j])
        by {
            assert(has_bill(ledger.bills, bill.prev[j]));
            let w = choose|w: int| 0 <= w < ledger.bills.len() && ledger.bills[w].id == bill.prev[j];
            assert(new_ledger.bills[w].id == bill.prev[j]);
        }
    }

    assert forall|i: int, j: int|
        0 <= i < post.ledgers.len() && 0 <= j < post.ledgers.len() && i != j
        implies #[trigger] post.ledgers[i].ledger_id != #[trigger] post.ledgers[j].ledger_id
    by {
        if i == idx { assert(post.ledgers[i].ledger_id == ledger.ledger_id); }
        if j == idx { assert(post.ledgers[j].ledger_id == ledger.ledger_id); }
    }

    other_ledgers_preserved(pre.ledgers, post.ledgers, idx, new_ledger);

    assert forall|i: int| 0 <= i < post.ledgers.len()
        implies ledger_ids_in_generated(#[trigger] post.ledgers[i], post.generated_ids)
    by {
        if i == idx {
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
        } else {
            assert(ledger_ids_in_generated(pre.ledgers[i], pre.generated_ids));
        }
    }
}

// ---------------------------------------------------------------------------
// add_bill preserves invariant
// ---------------------------------------------------------------------------

pub proof fn add_bill_preserves(
    pre: WorldModel, post: WorldModel, ledger_id: Id, bill: BillModel,
)
    requires
        state_machine_invariant(pre),
        add_bill(pre, post, ledger_id, bill),
    ensures
        state_machine_invariant(post),
{
    let idx = find_ledger(pre.ledgers, ledger_id);
    let ledger = pre.ledgers[idx];
    let new_ledger = LedgerModel { bills: ledger.bills.push(bill), ..ledger };

    // Bill IDs unique.
    assert forall|i: int, j: int|
        0 <= i < new_ledger.bills.len() && 0 <= j < new_ledger.bills.len() && i != j
        implies #[trigger] new_ledger.bills[i].id != #[trigger] new_ledger.bills[j].id
    by {
        if i < ledger.bills.len() && j < ledger.bills.len() {
        } else if i == ledger.bills.len() as int {
            assert(!pre.generated_ids.contains(bill.id));
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
            assert(pre.generated_ids.contains(ledger.bills[j].id));
        } else {
            assert(!pre.generated_ids.contains(bill.id));
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
            assert(pre.generated_ids.contains(ledger.bills[i].id));
        }
    }

    // All bills well-formed in new ledger.
    assert forall|i: int| 0 <= i < new_ledger.bills.len()
        implies bill_well_formed(
            #[trigger] new_ledger.bills[i], new_ledger.users, new_ledger.devices, new_ledger.bills,
        )
    by {
        let b = new_ledger.bills[i];
        if i < ledger.bills.len() as int {
            // Old bill: prev refs still valid (bills only grew).
            assert forall|j: int| 0 <= j < b.prev.len()
                implies has_bill(new_ledger.bills, #[trigger] b.prev[j])
            by {
                assert(has_bill(ledger.bills, b.prev[j]));
                let w = choose|w: int| 0 <= w < ledger.bills.len() && ledger.bills[w].id == b.prev[j];
                assert(new_ledger.bills[w].id == b.prev[j]);
            }
        } else {
            // New bill: prev refs valid (point to pre bills, which are prefix of post bills).
            assert forall|j: int| 0 <= j < b.prev.len()
                implies has_bill(new_ledger.bills, #[trigger] b.prev[j])
            by {
                assert(has_bill(ledger.bills, b.prev[j]));
                let w = choose|w: int| 0 <= w < ledger.bills.len() && ledger.bills[w].id == b.prev[j];
                assert(new_ledger.bills[w].id == b.prev[j]);
            }
        }
    }

    assert forall|i: int, j: int|
        0 <= i < post.ledgers.len() && 0 <= j < post.ledgers.len() && i != j
        implies #[trigger] post.ledgers[i].ledger_id != #[trigger] post.ledgers[j].ledger_id
    by {
        if i == idx { assert(post.ledgers[i].ledger_id == ledger.ledger_id); }
        if j == idx { assert(post.ledgers[j].ledger_id == ledger.ledger_id); }
    }

    other_ledgers_preserved(pre.ledgers, post.ledgers, idx, new_ledger);

    assert forall|i: int| 0 <= i < post.ledgers.len()
        implies ledger_ids_in_generated(#[trigger] post.ledgers[i], post.generated_ids)
    by {
        if i == idx {
            assert(ledger_ids_in_generated(ledger, pre.generated_ids));
        } else {
            assert(ledger_ids_in_generated(pre.ledgers[i], pre.generated_ids));
        }
    }
}

}
