// Layer 2: Proofs that each device transition preserves device_invariant.

use super::spec::*;
use crate::ledger::spec::*;
use vstd::prelude::*;

verus! {

/// Helper: updating one ledger in a sequence preserves invariants of others.
proof fn other_ledgers_invariant(
    pre: Seq<LedgerStateSpec>,
    post: Seq<LedgerStateSpec>,
    idx: int,
    new_ledger: LedgerStateSpec,
)
    requires
        0 <= idx < pre.len(),
        post == pre.update(idx, new_ledger),
        forall|i: int| 0 <= i < pre.len() ==> ledger_invariant(#[trigger] pre[i]),
        ledger_invariant(new_ledger),
    ensures
        forall|i: int| 0 <= i < post.len() ==> ledger_invariant(#[trigger] post[i]),
{
    assert forall|i: int| 0 <= i < post.len()
        implies ledger_invariant(#[trigger] post[i])
    by {
        if i == idx { assert(post[i] == new_ledger); }
        else { assert(post[i] == pre[i]); }
    }
}

/// Helper: updating one ledger preserves ledger_ids_unique (same ledger_id).
proof fn update_preserves_ledger_ids_unique(
    pre: Seq<LedgerStateSpec>,
    idx: int,
    new_ledger: LedgerStateSpec,
)
    requires
        0 <= idx < pre.len(),
        ledger_ids_unique(pre),
        new_ledger.ledger_id == pre[idx].ledger_id,
    ensures
        ledger_ids_unique(pre.update(idx, new_ledger)),
{
    assert forall|i: int, j: int|
        0 <= i < pre.update(idx, new_ledger).len()
        && 0 <= j < pre.update(idx, new_ledger).len()
        && i != j
        implies #[trigger] pre.update(idx, new_ledger)[i].ledger_id
            != #[trigger] pre.update(idx, new_ledger)[j].ledger_id
    by {
        if i == idx { assert(pre.update(idx, new_ledger)[i].ledger_id == pre[idx].ledger_id); }
        if j == idx { assert(pre.update(idx, new_ledger)[j].ledger_id == pre[idx].ledger_id); }
    }
}

pub proof fn create_ledger_preserves(
    pre: DeviceStateSpec, post: DeviceStateSpec,
    ledger_id: u128, name: Seq<u8>, currency: Seq<u8>, created_at: i64,
)
    requires
        device_invariant(pre),
        create_ledger(pre, post, ledger_id, name, currency, created_at),
    ensures
        device_invariant(post),
{
    // Ledger IDs unique: new ledger_id not in pre.
    assert forall|i: int, j: int|
        0 <= i < post.ledgers.len() && 0 <= j < post.ledgers.len() && i != j
        implies #[trigger] post.ledgers[i].ledger_id != #[trigger] post.ledgers[j].ledger_id
    by {
        if i == pre.ledgers.len() as int {
            assert(has_ledger(pre.ledgers, post.ledgers[j].ledger_id));
        } else if j == pre.ledgers.len() as int {
            assert(has_ledger(pre.ledgers, post.ledgers[i].ledger_id));
        }
    }

    // Every ledger well-formed: old ones unchanged, new one is empty.
    assert forall|i: int| 0 <= i < post.ledgers.len()
        implies ledger_invariant(#[trigger] post.ledgers[i])
    by {
        if i < pre.ledgers.len() as int {
            assert(post.ledgers[i] == pre.ledgers[i]);
        }
    }
}

pub proof fn device_add_user_preserves(
    pre: DeviceStateSpec, post: DeviceStateSpec,
    ledger_id: u128, user: UserSpec,
)
    requires
        device_invariant(pre),
        device_add_user(pre, post, ledger_id, user),
    ensures
        device_invariant(post),
{
    let idx = find_ledger(pre.ledgers, ledger_id);
    let pre_ledger = pre.ledgers[idx];
    let post_ledger = LedgerStateSpec { users: pre_ledger.users.push(user), ..pre_ledger };
    crate::ledger::proof::add_user_preserves(pre_ledger, post_ledger, user);
    update_preserves_ledger_ids_unique(pre.ledgers, idx, post_ledger);
    other_ledgers_invariant(pre.ledgers, post.ledgers, idx, post_ledger);
}

pub proof fn device_add_device_preserves(
    pre: DeviceStateSpec, post: DeviceStateSpec,
    ledger_id: u128, device: DeviceSpec,
)
    requires
        device_invariant(pre),
        device_add_device(pre, post, ledger_id, device),
    ensures
        device_invariant(post),
{
    let idx = find_ledger(pre.ledgers, ledger_id);
    let pre_ledger = pre.ledgers[idx];
    let post_ledger = LedgerStateSpec { devices: pre_ledger.devices.push(device), ..pre_ledger };
    crate::ledger::proof::add_device_preserves(pre_ledger, post_ledger, device);
    update_preserves_ledger_ids_unique(pre.ledgers, idx, post_ledger);
    other_ledgers_invariant(pre.ledgers, post.ledgers, idx, post_ledger);
}

pub proof fn device_add_bill_preserves(
    pre: DeviceStateSpec, post: DeviceStateSpec,
    ledger_id: u128, bill: BillSpec,
)
    requires
        device_invariant(pre),
        device_add_bill(pre, post, ledger_id, bill),
    ensures
        device_invariant(post),
{
    let idx = find_ledger(pre.ledgers, ledger_id);
    let pre_ledger = pre.ledgers[idx];
    let post_ledger = LedgerStateSpec { bills: pre_ledger.bills.push(bill), ..pre_ledger };
    crate::ledger::proof::add_bill_preserves(pre_ledger, post_ledger, bill);
    update_preserves_ledger_ids_unique(pre.ledgers, idx, post_ledger);
    other_ledgers_invariant(pre.ledgers, post.ledgers, idx, post_ledger);
}

}
