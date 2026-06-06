// Proof utilities for the verified ledger state machine.
// Proves that each transition preserves the state machine invariant.

use super::spec::*;
use vstd::prelude::*;

verus! {

/// Proof: init produces a state satisfying the invariant.
pub proof fn init_preserves(post: LedgerModel, ledger_id: u64)
    requires
        init(post, ledger_id),
    ensures
        state_machine_invariant(post),
{
    // Empty sequences trivially satisfy uniqueness and the forall over bills.
}

/// Proof: add_user preserves the invariant.
pub proof fn add_user_preserves(pre: LedgerModel, post: LedgerModel, user: UserModel)
    requires
        state_machine_invariant(pre),
        add_user(pre, post, user),
    ensures
        state_machine_invariant(post),
{
    // User ID uniqueness: pre has unique IDs, new user is fresh.
    assert forall|i: int, j: int|
        0 <= i < post.users.len() && 0 <= j < post.users.len() && i != j
        implies #[trigger] post.users[i].user_id != #[trigger] post.users[j].user_id
    by {
        if i < pre.users.len() && j < pre.users.len() {
            // Both old — unique by pre invariant.
        } else if i == pre.users.len() as int {
            // i is the new user. j is old. New user ID is fresh.
            assert(post.users[i].user_id == user.user_id);
            assert(has_user(pre.users, post.users[j].user_id));
        } else if j == pre.users.len() as int {
            assert(post.users[j].user_id == user.user_id);
            assert(has_user(pre.users, post.users[i].user_id));
        }
    }

    // Bill well-formedness: bills unchanged, users grew.
    // All existing share references still valid (users only grows).
    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let bill = post.bills[i];
        // Shares reference known users: if has_user(pre.users, id) then has_user(post.users, id).
        assert forall|k: int| 0 <= k < bill.payers.len()
            implies has_user(post.users, #[trigger] bill.payers[k].0)
        by {
            assert(has_user(pre.users, bill.payers[k].0));
            // pre.users is a prefix of post.users, so membership is preserved.
            let witness = choose|w: int| 0 <= w < pre.users.len() && pre.users[w].user_id == bill.payers[k].0;
            assert(post.users[witness].user_id == bill.payers[k].0);
        }
        assert forall|k: int| 0 <= k < bill.payees.len()
            implies has_user(post.users, #[trigger] bill.payees[k].0)
        by {
            assert(has_user(pre.users, bill.payees[k].0));
            let witness = choose|w: int| 0 <= w < pre.users.len() && pre.users[w].user_id == bill.payees[k].0;
            assert(post.users[witness].user_id == bill.payees[k].0);
        }
        // Device and prev references unchanged.
        assert(has_device(post.devices, bill.created_by_device));
        assert forall|j: int| 0 <= j < bill.prev.len()
            implies has_bill(post.bills, #[trigger] bill.prev[j])
        by {
            assert(has_bill(pre.bills, bill.prev[j]));
            let witness = choose|w: int| 0 <= w < pre.bills.len() && pre.bills[w].id == bill.prev[j];
            assert(post.bills[witness].id == bill.prev[j]);
        }
    }
}

/// Proof: add_device preserves the invariant.
pub proof fn add_device_preserves(pre: LedgerModel, post: LedgerModel, device: DeviceModel)
    requires
        state_machine_invariant(pre),
        add_device(pre, post, device),
    ensures
        state_machine_invariant(post),
{
    // Device ID uniqueness.
    assert forall|i: int, j: int|
        0 <= i < post.devices.len() && 0 <= j < post.devices.len() && i != j
        implies #[trigger] post.devices[i].device_id != #[trigger] post.devices[j].device_id
    by {
        if i < pre.devices.len() && j < pre.devices.len() {
        } else if i == pre.devices.len() as int {
            assert(post.devices[i].device_id == device.device_id);
            assert(has_device(pre.devices, post.devices[j].device_id));
        } else if j == pre.devices.len() as int {
            assert(post.devices[j].device_id == device.device_id);
            assert(has_device(pre.devices, post.devices[i].device_id));
        }
    }

    // Bill well-formedness: devices grew, references still valid.
    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let bill = post.bills[i];
        // Device reference: if has_device(pre, id) then has_device(post, id).
        assert(has_device(pre.devices, bill.created_by_device));
        let witness = choose|w: int| 0 <= w < pre.devices.len() && pre.devices[w].device_id == bill.created_by_device;
        assert(post.devices[witness].device_id == bill.created_by_device);
        // Bill prev references unchanged.
        assert forall|j: int| 0 <= j < bill.prev.len()
            implies has_bill(post.bills, #[trigger] bill.prev[j])
        by {
            assert(has_bill(pre.bills, bill.prev[j]));
            let witness = choose|w: int| 0 <= w < pre.bills.len() && pre.bills[w].id == bill.prev[j];
            assert(post.bills[witness].id == bill.prev[j]);
        }
    }
}

/// Proof: add_bill preserves the invariant.
pub proof fn add_bill_preserves(pre: LedgerModel, post: LedgerModel, bill: BillModel)
    requires
        state_machine_invariant(pre),
        add_bill(pre, post, bill),
    ensures
        state_machine_invariant(post),
{
    // Bill ID uniqueness: pre has unique IDs, new bill ID is fresh.
    assert forall|i: int, j: int|
        0 <= i < post.bills.len() && 0 <= j < post.bills.len() && i != j
        implies #[trigger] post.bills[i].id != #[trigger] post.bills[j].id
    by {
        if i < pre.bills.len() && j < pre.bills.len() {
        } else if i == pre.bills.len() as int {
            assert(post.bills[i].id == bill.id);
            assert(!has_bill(pre.bills, bill.id));
            assert(has_bill(pre.bills, post.bills[j].id));
        } else if j == pre.bills.len() as int {
            assert(post.bills[j].id == bill.id);
            assert(!has_bill(pre.bills, bill.id));
            assert(has_bill(pre.bills, post.bills[i].id));
        }
    }

    // Every bill (old and new) is well-formed in the post state.
    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let b = post.bills[i];
        if i < pre.bills.len() as int {
            // Old bill: was well-formed in pre. Users/devices unchanged.
            // prev references: pre.bills is a prefix of post.bills.
            assert forall|j: int| 0 <= j < b.prev.len()
                implies has_bill(post.bills, #[trigger] b.prev[j])
            by {
                assert(has_bill(pre.bills, b.prev[j]));
                let witness = choose|w: int| 0 <= w < pre.bills.len() && pre.bills[w].id == b.prev[j];
                assert(post.bills[witness].id == b.prev[j]);
            }
        } else {
            // New bill: well-formed by add_bill predicate.
            // prev references point to pre.bills, which is a prefix of post.bills.
            assert forall|j: int| 0 <= j < b.prev.len()
                implies has_bill(post.bills, #[trigger] b.prev[j])
            by {
                assert(has_bill(pre.bills, b.prev[j]));
                let witness = choose|w: int| 0 <= w < pre.bills.len() && pre.bills[w].id == b.prev[j];
                assert(post.bills[witness].id == b.prev[j]);
            }
        }
    }
}

}
