// Layer 3: Proofs that each world transition preserves world_invariant.

use super::spec::*;
use crate::device::spec::*;
use crate::ledger::spec::*;
use vstd::prelude::*;

verus! {

/// Helper: updating one device preserves device_invariant for others.
proof fn other_devices_invariant(
    pre: Seq<DeviceStateSpec>,
    post: Seq<DeviceStateSpec>,
    idx: int,
    new_device: DeviceStateSpec,
)
    requires
        0 <= idx < pre.len(),
        post == pre.update(idx, new_device),
        forall|i: int| 0 <= i < pre.len() ==> device_invariant(#[trigger] pre[i]),
        device_invariant(new_device),
    ensures
        forall|i: int| 0 <= i < post.len() ==> device_invariant(#[trigger] post[i]),
{
    assert forall|i: int| 0 <= i < post.len()
        implies device_invariant(#[trigger] post[i])
    by {
        if i == idx { assert(post[i] == new_device); }
        else { assert(post[i] == pre[i]); }
    }
}

/// Helper: updating one device (same device_id) preserves device_ids_unique_in_world.
proof fn update_preserves_device_ids_unique(
    pre: Seq<DeviceStateSpec>,
    idx: int,
    new_device: DeviceStateSpec,
)
    requires
        0 <= idx < pre.len(),
        device_ids_unique_in_world(pre),
        new_device.device_id == pre[idx].device_id,
    ensures
        device_ids_unique_in_world(pre.update(idx, new_device)),
{
    assert forall|i: int, j: int|
        0 <= i < pre.update(idx, new_device).len()
        && 0 <= j < pre.update(idx, new_device).len()
        && i != j
        implies #[trigger] pre.update(idx, new_device)[i].device_id
            != #[trigger] pre.update(idx, new_device)[j].device_id
    by {
        if i == idx { assert(pre.update(idx, new_device)[i].device_id == pre[idx].device_id); }
        if j == idx { assert(pre.update(idx, new_device)[j].device_id == pre[idx].device_id); }
    }
}

pub proof fn register_device_preserves(
    pre: WorldSpec, post: WorldSpec, device_id: Seq<u8>,
)
    requires
        world_invariant(pre),
        register_device(pre, post, device_id),
    ensures
        world_invariant(post),
{
    // Device IDs unique: new device_id is fresh (not in generated, all existing IDs are in generated).
    assert forall|i: int, j: int|
        0 <= i < post.devices.len() && 0 <= j < post.devices.len() && i != j
        implies #[trigger] post.devices[i].device_id != #[trigger] post.devices[j].device_id
    by {
        if i == pre.devices.len() as int {
            assert(!pre.ulid_state.generated.contains(device_id));
            assert(pre.ulid_state.generated.contains(pre.devices[j].device_id));
        } else if j == pre.devices.len() as int {
            assert(!pre.ulid_state.generated.contains(device_id));
            assert(pre.ulid_state.generated.contains(pre.devices[i].device_id));
        }
    }

    // Each device well-formed.
    assert forall|i: int| 0 <= i < post.devices.len()
        implies device_invariant(#[trigger] post.devices[i])
    by {
        if i < pre.devices.len() as int { assert(post.devices[i] == pre.devices[i]); }
    }

    // All IDs tracked: old IDs still in superset, new device_id inserted.
    assert(all_ids_tracked(post)) by {
        assert forall|i: int| 0 <= i < post.devices.len()
            implies post.ulid_state.generated.contains(#[trigger] post.devices[i].device_id)
        by {
            if i < pre.devices.len() as int {
                assert(pre.ulid_state.generated.contains(pre.devices[i].device_id));
            }
        }
    }
}

pub proof fn world_create_ledger_preserves(
    pre: WorldSpec, post: WorldSpec,
    device_id: Seq<u8>, ledger_id: Seq<u8>,
    name: Seq<u8>, currency: Seq<u8>, created_at: i64,
)
    requires
        world_invariant(pre),
        world_create_ledger(pre, post, device_id, ledger_id, name, currency, created_at),
    ensures
        world_invariant(post),
{
    let didx = find_device(pre.devices, device_id);
    let pre_device = pre.devices[didx];

    // The new ledger_id is fresh in the ULID state, so it can't collide
    // with any existing ledger_id on the device.
    // Prove device_invariant for the updated device.
    assert(!has_ledger(pre_device.ledgers, ledger_id)) by {
        if has_ledger(pre_device.ledgers, ledger_id) {
            let k = choose|k: int| 0 <= k < pre_device.ledgers.len()
                && pre_device.ledgers[k].ledger_id == ledger_id;
            // pre_device.ledgers[k].ledger_id == ledger_id, and it's tracked.
            // But ulid_fresh says ledger_id is NOT in generated. Contradiction.
            assert(pre.ulid_state.generated.contains(ledger_id));
        }
    }

    let post_device = post.devices[didx];
    crate::device::proof::create_ledger_preserves(pre_device, post_device, ledger_id, name, currency, created_at);
    update_preserves_device_ids_unique(pre.devices, didx, post_device);
    other_devices_invariant(pre.devices, post.devices, didx, post_device);

    // All IDs tracked.
    assert(all_ids_tracked(post)) by {
        assert forall|i: int| 0 <= i < post.devices.len()
            implies post.ulid_state.generated.contains(#[trigger] post.devices[i].device_id)
        by {
            if i == didx { assert(post.devices[i].device_id == pre_device.device_id); }
            assert(pre.ulid_state.generated.contains(pre.devices[i].device_id));
        }
        assert forall|i: int, j: int|
            0 <= i < post.devices.len()
            && 0 <= j < (#[trigger] post.devices[i]).ledgers.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].ledger_id)
        by {
            if i == didx && j == pre_device.ledgers.len() as int {
                // New ledger — its ID was just inserted.
            } else if i == didx {
                assert(post.devices[i].ledgers[j] == pre_device.ledgers[j]);
                assert(pre.ulid_state.generated.contains(pre_device.ledgers[j].ledger_id));
            } else {
                assert(post.devices[i] == pre.devices[i]);
            }
        }
    }
}

/// Helper: prove all_ids_tracked after updating one device and inserting one ID.
proof fn all_ids_tracked_after_device_update(
    pre: WorldSpec,
    post_devices: Seq<DeviceStateSpec>,
    post_ulid: UlidStateSpec,
    didx: int,
)
    requires
        all_ids_tracked(pre),
        0 <= didx < pre.devices.len(),
        post_devices == pre.devices.update(didx, post_devices[didx]),
        post_devices[didx].device_id == pre.devices[didx].device_id,
        // post_ulid is a superset of pre.ulid_state.generated.
        forall|id: Seq<u8>| pre.ulid_state.generated.contains(id) ==>
            post_ulid.generated.contains(id),
    ensures
        forall|i: int| 0 <= i < post_devices.len() ==>
            post_ulid.generated.contains(#[trigger] post_devices[i].device_id),
{
    assert forall|i: int| 0 <= i < post_devices.len()
        implies post_ulid.generated.contains(#[trigger] post_devices[i].device_id)
    by {
        if i == didx {
            assert(post_devices[i].device_id == pre.devices[didx].device_id);
        }
        assert(pre.ulid_state.generated.contains(pre.devices[i].device_id));
    }
}

pub proof fn world_add_user_preserves(
    pre: WorldSpec, post: WorldSpec,
    device_id: Seq<u8>, ledger_id: Seq<u8>, user: UserSpec,
)
    requires
        world_invariant(pre),
        world_add_user(pre, post, device_id, ledger_id, user),
    ensures
        world_invariant(post),
{
    let didx = find_device(pre.devices, device_id);
    let pre_dev = pre.devices[didx];
    let lidx = find_ledger(pre_dev.ledgers, ledger_id);
    let pre_ledger = pre_dev.ledgers[lidx];
    let post_ledger = LedgerStateSpec { users: pre_ledger.users.push(user), ..pre_ledger };
    let post_dev = DeviceStateSpec {
        ledgers: pre_dev.ledgers.update(lidx, post_ledger),
        ..pre_dev
    };

    assert(!has_user(pre_ledger.users, user.user_id)) by {
        if has_user(pre_ledger.users, user.user_id) {
            let k = choose|k: int| 0 <= k < pre_ledger.users.len()
                && pre_ledger.users[k].user_id == user.user_id;
            assert(pre.ulid_state.generated.contains(user.user_id));
        }
    }

    crate::device::proof::device_add_user_preserves(pre_dev, post_dev, ledger_id, user);
    update_preserves_device_ids_unique(pre.devices, didx, post_dev);
    other_devices_invariant(pre.devices, post.devices, didx, post_dev);
    all_ids_tracked_after_device_update(pre, post.devices, post.ulid_state, didx);

    assert(all_ids_tracked(post)) by {
        assert forall|i: int, j: int|
            0 <= i < post.devices.len()
            && 0 <= j < (#[trigger] post.devices[i]).ledgers.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].ledger_id)
        by {
            if i == didx { assert(pre.ulid_state.generated.contains(pre_dev.ledgers[j].ledger_id)); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].users.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].users[k].user_id)
        by {
            if i == didx && j == lidx && k == pre_ledger.users.len() as int { }
            else if i == didx && j == lidx { assert(post.devices[i].ledgers[j].users[k] == pre_ledger.users[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].devices.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].devices[k].node_id)
        by {
            if i == didx && j == lidx { assert(post.devices[i].ledgers[j].devices[k] == pre_ledger.devices[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].bills.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].bills[k].id)
        by {
            if i == didx && j == lidx { assert(post.devices[i].ledgers[j].bills[k] == pre_ledger.bills[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
    }
}

pub proof fn world_add_device_to_ledger_preserves(
    pre: WorldSpec, post: WorldSpec,
    device_id: Seq<u8>, ledger_id: Seq<u8>, new_device: DeviceSpec,
)
    requires
        world_invariant(pre),
        world_add_device_to_ledger(pre, post, device_id, ledger_id, new_device),
    ensures
        world_invariant(post),
{
    let didx = find_device(pre.devices, device_id);
    let pre_dev = pre.devices[didx];
    let lidx = find_ledger(pre_dev.ledgers, ledger_id);
    let pre_ledger = pre_dev.ledgers[lidx];
    let post_ledger = LedgerStateSpec { devices: pre_ledger.devices.push(new_device), ..pre_ledger };
    let post_dev = DeviceStateSpec {
        ledgers: pre_dev.ledgers.update(lidx, post_ledger),
        ..pre_dev
    };

    assert(!has_device(pre_ledger.devices, new_device.node_id)) by {
        if has_device(pre_ledger.devices, new_device.node_id) {
            let k = choose|k: int| 0 <= k < pre_ledger.devices.len()
                && pre_ledger.devices[k].node_id == new_device.node_id;
            assert(pre.ulid_state.generated.contains(new_device.node_id));
        }
    }

    crate::device::proof::device_add_device_preserves(pre_dev, post_dev, ledger_id, new_device);
    update_preserves_device_ids_unique(pre.devices, didx, post_dev);
    other_devices_invariant(pre.devices, post.devices, didx, post_dev);
    all_ids_tracked_after_device_update(pre, post.devices, post.ulid_state, didx);

    assert(all_ids_tracked(post)) by {
        assert forall|i: int, j: int|
            0 <= i < post.devices.len()
            && 0 <= j < (#[trigger] post.devices[i]).ledgers.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].ledger_id)
        by {
            if i == didx { assert(pre.ulid_state.generated.contains(pre_dev.ledgers[j].ledger_id)); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].users.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].users[k].user_id)
        by {
            if i == didx && j == lidx { assert(post.devices[i].ledgers[j].users[k] == pre_ledger.users[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].devices.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].devices[k].node_id)
        by {
            if i == didx && j == lidx && k == pre_ledger.devices.len() as int { }
            else if i == didx && j == lidx { assert(post.devices[i].ledgers[j].devices[k] == pre_ledger.devices[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].bills.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].bills[k].id)
        by {
            if i == didx && j == lidx { assert(post.devices[i].ledgers[j].bills[k] == pre_ledger.bills[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
    }
}

pub proof fn world_add_bill_preserves(
    pre: WorldSpec, post: WorldSpec,
    device_id: Seq<u8>, ledger_id: Seq<u8>, bill: BillSpec,
)
    requires
        world_invariant(pre),
        world_add_bill(pre, post, device_id, ledger_id, bill),
    ensures
        world_invariant(post),
{
    let didx = find_device(pre.devices, device_id);
    let pre_dev = pre.devices[didx];
    let lidx = find_ledger(pre_dev.ledgers, ledger_id);
    let pre_ledger = pre_dev.ledgers[lidx];
    let post_ledger = LedgerStateSpec { bills: pre_ledger.bills.push(bill), ..pre_ledger };
    let post_dev = DeviceStateSpec {
        ledgers: pre_dev.ledgers.update(lidx, post_ledger),
        ..pre_dev
    };

    assert(!has_bill(pre_ledger.bills, bill.id)) by {
        if has_bill(pre_ledger.bills, bill.id) {
            let k = choose|k: int| 0 <= k < pre_ledger.bills.len()
                && pre_ledger.bills[k].id == bill.id;
            assert(pre.ulid_state.generated.contains(bill.id));
        }
    }

    crate::device::proof::device_add_bill_preserves(pre_dev, post_dev, ledger_id, bill);
    update_preserves_device_ids_unique(pre.devices, didx, post_dev);
    other_devices_invariant(pre.devices, post.devices, didx, post_dev);
    all_ids_tracked_after_device_update(pre, post.devices, post.ulid_state, didx);

    assert(all_ids_tracked(post)) by {
        assert forall|i: int, j: int|
            0 <= i < post.devices.len()
            && 0 <= j < (#[trigger] post.devices[i]).ledgers.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].ledger_id)
        by {
            if i == didx { assert(pre.ulid_state.generated.contains(pre_dev.ledgers[j].ledger_id)); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].users.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].users[k].user_id)
        by {
            if i == didx && j == lidx { assert(post.devices[i].ledgers[j].users[k] == pre_ledger.users[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].devices.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].devices[k].node_id)
        by {
            if i == didx && j == lidx { assert(post.devices[i].ledgers[j].devices[k] == pre_ledger.devices[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
        assert forall|i: int, j: int, k: int|
            0 <= i < post.devices.len()
            && 0 <= j < post.devices[i].ledgers.len()
            && 0 <= k < post.devices[i].ledgers[j].bills.len()
            implies post.ulid_state.generated.contains(
                #[trigger] post.devices[i].ledgers[j].bills[k].id)
        by {
            if i == didx && j == lidx && k == pre_ledger.bills.len() as int { }
            else if i == didx && j == lidx { assert(post.devices[i].ledgers[j].bills[k] == pre_ledger.bills[k]); }
            else if i == didx { assert(post.devices[i].ledgers[j] == pre_dev.ledgers[j]); }
            else { assert(post.devices[i] == pre.devices[i]); }
        }
    }
}

}
