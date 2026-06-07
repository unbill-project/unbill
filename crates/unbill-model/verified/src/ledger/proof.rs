// Layer 1: Proofs that each transition preserves ledger_invariant.

use super::spec::*;
use vstd::prelude::*;

verus! {

pub proof fn init_preserves(
    post: LedgerStateSpec,
    ledger_id: Seq<u8>,
    name: Seq<u8>,
    currency: Seq<u8>,
    created_at: i64,
)
    requires init(post, ledger_id, name, currency, created_at),
    ensures ledger_invariant(post),
{}

pub proof fn add_user_preserves(
    pre: LedgerStateSpec, post: LedgerStateSpec, user: UserSpec,
)
    requires
        ledger_invariant(pre),
        add_user(pre, post, user),
    ensures
        ledger_invariant(post),
{
    // User IDs unique: fresh user_id not in pre.users.
    assert forall|i: int, j: int|
        0 <= i < post.users.len() && 0 <= j < post.users.len() && i != j
        implies #[trigger] post.users[i].user_id != #[trigger] post.users[j].user_id
    by {
        if i == pre.users.len() as int {
            assert(has_user(pre.users, post.users[j].user_id));
        } else if j == pre.users.len() as int {
            assert(has_user(pre.users, post.users[i].user_id));
        }
    }

    // Bills still well-formed: users grew, shares still reference known users.
    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let bill = post.bills[i];
        assert forall|k: int| 0 <= k < bill.payers.len()
            implies has_user(post.users, #[trigger] bill.payers[k].user_id)
        by {
            let w = choose|w: int| 0 <= w < pre.users.len()
                && pre.users[w].user_id == bill.payers[k].user_id;
            assert(post.users[w].user_id == bill.payers[k].user_id);
        }
        assert forall|k: int| 0 <= k < bill.payees.len()
            implies has_user(post.users, #[trigger] bill.payees[k].user_id)
        by {
            let w = choose|w: int| 0 <= w < pre.users.len()
                && pre.users[w].user_id == bill.payees[k].user_id;
            assert(post.users[w].user_id == bill.payees[k].user_id);
        }
        assert forall|j: int| 0 <= j < bill.prev.len()
            implies has_bill(post.bills, #[trigger] bill.prev[j])
        by {
            let w = choose|w: int| 0 <= w < pre.bills.len()
                && pre.bills[w].id == bill.prev[j];
            assert(post.bills[w].id == bill.prev[j]);
        }
    }
}

pub proof fn add_device_preserves(
    pre: LedgerStateSpec, post: LedgerStateSpec, device: DeviceSpec,
)
    requires
        ledger_invariant(pre),
        add_device(pre, post, device),
    ensures
        ledger_invariant(post),
{
    assert forall|i: int, j: int|
        0 <= i < post.devices.len() && 0 <= j < post.devices.len() && i != j
        implies #[trigger] post.devices[i].node_id != #[trigger] post.devices[j].node_id
    by {
        if i == pre.devices.len() as int {
            assert(has_device(pre.devices, post.devices[j].node_id));
        } else if j == pre.devices.len() as int {
            assert(has_device(pre.devices, post.devices[i].node_id));
        }
    }

    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let bill = post.bills[i];
        let w = choose|w: int| 0 <= w < pre.devices.len()
            && pre.devices[w].node_id == bill.created_by_device;
        assert(post.devices[w].node_id == bill.created_by_device);
        assert forall|j: int| 0 <= j < bill.prev.len()
            implies has_bill(post.bills, #[trigger] bill.prev[j])
        by {
            let w = choose|w: int| 0 <= w < pre.bills.len()
                && pre.bills[w].id == bill.prev[j];
            assert(post.bills[w].id == bill.prev[j]);
        }
    }
}

pub proof fn add_bill_preserves(
    pre: LedgerStateSpec, post: LedgerStateSpec, bill: BillSpec,
)
    requires
        ledger_invariant(pre),
        add_bill(pre, post, bill),
    ensures
        ledger_invariant(post),
{
    // Bill IDs unique.
    assert forall|i: int, j: int|
        0 <= i < post.bills.len() && 0 <= j < post.bills.len() && i != j
        implies #[trigger] post.bills[i].id != #[trigger] post.bills[j].id
    by {
        if i == pre.bills.len() as int {
            assert(!has_bill(pre.bills, bill.id));
            assert(has_bill(pre.bills, post.bills[j].id));
        } else if j == pre.bills.len() as int {
            assert(!has_bill(pre.bills, bill.id));
            assert(has_bill(pre.bills, post.bills[i].id));
        }
    }

    // All bills well-formed in post.
    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let b = post.bills[i];
        assert forall|j: int| 0 <= j < b.prev.len()
            implies has_bill(post.bills, #[trigger] b.prev[j])
        by {
            assert(has_bill(pre.bills, b.prev[j]));
            let w = choose|w: int| 0 <= w < pre.bills.len()
                && pre.bills[w].id == b.prev[j];
            assert(post.bills[w].id == b.prev[j]);
        }
    }
}

}
