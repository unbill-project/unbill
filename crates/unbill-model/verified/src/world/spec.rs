// Layer 3: WorldModel — the global system state.
// Contains all devices and the ULID uniqueness state.

use crate::device::spec::*;
use crate::ledger::spec::*;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Spec types
// ---------------------------------------------------------------------------

/// Tracks all IDs ever generated. Axiomatically, ULID generation
/// always produces an ID not in this set.
pub struct UlidStateSpec {
    pub generated: Set<Seq<u8>>,
}

pub struct WorldSpec {
    pub devices: Seq<DeviceStateSpec>,
    pub ulid_state: UlidStateSpec,
}

// ---------------------------------------------------------------------------
// Helper predicates
// ---------------------------------------------------------------------------

pub open spec fn has_device_in_world(
    devices: Seq<DeviceStateSpec>, device_id: Seq<u8>,
) -> bool {
    exists|i: int| 0 <= i < devices.len()
        && #[trigger] devices[i].device_id == device_id
}

pub open spec fn find_device(
    devices: Seq<DeviceStateSpec>, device_id: Seq<u8>,
) -> int
    recommends has_device_in_world(devices, device_id),
{
    choose|i: int| 0 <= i < devices.len() && devices[i].device_id == device_id
}

pub open spec fn device_ids_unique_in_world(devices: Seq<DeviceStateSpec>) -> bool {
    forall|i: int, j: int|
        0 <= i < devices.len() && 0 <= j < devices.len() && i != j
        ==> #[trigger] devices[i].device_id != #[trigger] devices[j].device_id
}

/// All IDs across all devices and ledgers are tracked by the ULID state.
pub open spec fn all_ids_tracked(world: WorldSpec) -> bool {
    // Device IDs tracked.
    &&& forall|i: int| 0 <= i < world.devices.len() ==>
        world.ulid_state.generated.contains(#[trigger] world.devices[i].device_id)
    // All ledger IDs tracked.
    &&& forall|i: int, j: int|
        0 <= i < world.devices.len()
        && 0 <= j < (#[trigger] world.devices[i]).ledgers.len()
        ==> world.ulid_state.generated.contains(
            #[trigger] world.devices[i].ledgers[j].ledger_id)
    // All user IDs tracked.
    &&& forall|i: int, j: int, k: int|
        0 <= i < world.devices.len()
        && 0 <= j < world.devices[i].ledgers.len()
        && 0 <= k < world.devices[i].ledgers[j].users.len()
        ==> world.ulid_state.generated.contains(
            #[trigger] world.devices[i].ledgers[j].users[k].user_id)
    // All device-in-ledger IDs tracked.
    &&& forall|i: int, j: int, k: int|
        0 <= i < world.devices.len()
        && 0 <= j < world.devices[i].ledgers.len()
        && 0 <= k < world.devices[i].ledgers[j].devices.len()
        ==> world.ulid_state.generated.contains(
            #[trigger] world.devices[i].ledgers[j].devices[k].node_id)
    // All bill IDs tracked.
    &&& forall|i: int, j: int, k: int|
        0 <= i < world.devices.len()
        && 0 <= j < world.devices[i].ledgers.len()
        && 0 <= k < world.devices[i].ledgers[j].bills.len()
        ==> world.ulid_state.generated.contains(
            #[trigger] world.devices[i].ledgers[j].bills[k].id)
}

/// ULID uniqueness property: the generator has produced all tracked IDs,
/// and any fresh ID from the generator will not collide with any existing ID.
/// This is the axiom we trust about Ulid::new().
pub open spec fn ulid_fresh(ulid_state: UlidStateSpec, new_id: Seq<u8>) -> bool {
    !ulid_state.generated.contains(new_id)
}

// ---------------------------------------------------------------------------
// State machine invariant
// ---------------------------------------------------------------------------

pub open spec fn world_invariant(world: WorldSpec) -> bool {
    &&& device_ids_unique_in_world(world.devices)
    &&& forall|i: int| 0 <= i < world.devices.len() ==>
        device_invariant(#[trigger] world.devices[i])
    &&& all_ids_tracked(world)
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

/// Register a new device in the world.
pub open spec fn register_device(
    pre: WorldSpec, post: WorldSpec, device_id: Seq<u8>,
) -> bool {
    &&& ulid_fresh(pre.ulid_state, device_id)
    &&& !has_device_in_world(pre.devices, device_id)
    &&& post == WorldSpec {
        devices: pre.devices.push(DeviceStateSpec {
            device_id: device_id,
            ledgers: Seq::empty(),
        }),
        ulid_state: UlidStateSpec {
            generated: pre.ulid_state.generated.insert(device_id),
        },
    }
}

/// Create a ledger on a specific device.
pub open spec fn world_create_ledger(
    pre: WorldSpec, post: WorldSpec,
    device_id: Seq<u8>,
    ledger_id: Seq<u8>,
    name: Seq<u8>,
    currency: Seq<u8>,
    created_at: i64,
) -> bool {
    &&& has_device_in_world(pre.devices, device_id)
    &&& ulid_fresh(pre.ulid_state, ledger_id)
    &&& {
        let idx = find_device(pre.devices, device_id);
        let pre_device = pre.devices[idx];
        &&& create_ledger(pre_device, DeviceStateSpec {
            ledgers: pre_device.ledgers.push(LedgerStateSpec {
                ledger_id: ledger_id,
                schema_version: 1,
                name: name,
                currency: currency,
                created_at: created_at,
                users: Seq::empty(),
                bills: Seq::empty(),
                devices: Seq::empty(),
            }),
            ..pre_device
        }, ledger_id, name, currency, created_at)
        &&& post == WorldSpec {
            devices: pre.devices.update(idx, DeviceStateSpec {
                ledgers: pre_device.ledgers.push(LedgerStateSpec {
                    ledger_id: ledger_id,
                    schema_version: 1,
                    name: name,
                    currency: currency,
                    created_at: created_at,
                    users: Seq::empty(),
                    bills: Seq::empty(),
                    devices: Seq::empty(),
                }),
                ..pre_device
            }),
            ulid_state: UlidStateSpec {
                generated: pre.ulid_state.generated.insert(ledger_id),
            },
        }
    }
}

/// Add a user on a specific device + ledger.
pub open spec fn world_add_user(
    pre: WorldSpec, post: WorldSpec,
    device_id: Seq<u8>, ledger_id: Seq<u8>, user: UserSpec,
) -> bool {
    &&& has_device_in_world(pre.devices, device_id)
    &&& ulid_fresh(pre.ulid_state, user.user_id)
    &&& {
        let didx = find_device(pre.devices, device_id);
        let pre_device = pre.devices[didx];
        let lidx = find_ledger(pre_device.ledgers, ledger_id);
        let pre_ledger = pre_device.ledgers[lidx];
        &&& has_ledger(pre_device.ledgers, ledger_id)
        &&& add_user(pre_ledger, LedgerStateSpec { users: pre_ledger.users.push(user), ..pre_ledger }, user)
        &&& post == WorldSpec {
            devices: pre.devices.update(didx, DeviceStateSpec {
                ledgers: pre_device.ledgers.update(lidx, LedgerStateSpec {
                    users: pre_ledger.users.push(user),
                    ..pre_ledger
                }),
                ..pre_device
            }),
            ulid_state: UlidStateSpec {
                generated: pre.ulid_state.generated.insert(user.user_id),
            },
        }
    }
}

/// Add a device-in-ledger on a specific device + ledger.
pub open spec fn world_add_device_to_ledger(
    pre: WorldSpec, post: WorldSpec,
    device_id: Seq<u8>, ledger_id: Seq<u8>, new_device: DeviceSpec,
) -> bool {
    &&& has_device_in_world(pre.devices, device_id)
    &&& ulid_fresh(pre.ulid_state, new_device.node_id)
    &&& {
        let didx = find_device(pre.devices, device_id);
        let pre_device = pre.devices[didx];
        let lidx = find_ledger(pre_device.ledgers, ledger_id);
        let pre_ledger = pre_device.ledgers[lidx];
        &&& has_ledger(pre_device.ledgers, ledger_id)
        &&& add_device(pre_ledger, LedgerStateSpec { devices: pre_ledger.devices.push(new_device), ..pre_ledger }, new_device)
        &&& post == WorldSpec {
            devices: pre.devices.update(didx, DeviceStateSpec {
                ledgers: pre_device.ledgers.update(lidx, LedgerStateSpec {
                    devices: pre_ledger.devices.push(new_device),
                    ..pre_ledger
                }),
                ..pre_device
            }),
            ulid_state: UlidStateSpec {
                generated: pre.ulid_state.generated.insert(new_device.node_id),
            },
        }
    }
}

/// Add a bill on a specific device + ledger.
pub open spec fn world_add_bill(
    pre: WorldSpec, post: WorldSpec,
    device_id: Seq<u8>, ledger_id: Seq<u8>, bill: BillSpec,
) -> bool {
    &&& has_device_in_world(pre.devices, device_id)
    &&& ulid_fresh(pre.ulid_state, bill.id)
    &&& {
        let didx = find_device(pre.devices, device_id);
        let pre_device = pre.devices[didx];
        let lidx = find_ledger(pre_device.ledgers, ledger_id);
        let pre_ledger = pre_device.ledgers[lidx];
        &&& has_ledger(pre_device.ledgers, ledger_id)
        &&& add_bill(pre_ledger, LedgerStateSpec { bills: pre_ledger.bills.push(bill), ..pre_ledger }, bill)
        &&& post == WorldSpec {
            devices: pre.devices.update(didx, DeviceStateSpec {
                ledgers: pre_device.ledgers.update(lidx, LedgerStateSpec {
                    bills: pre_ledger.bills.push(bill),
                    ..pre_ledger
                }),
                ..pre_device
            }),
            ulid_state: UlidStateSpec {
                generated: pre.ulid_state.generated.insert(bill.id),
            },
        }
    }
}

}
