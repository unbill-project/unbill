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

}
