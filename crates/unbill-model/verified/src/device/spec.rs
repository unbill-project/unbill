// Layer 2: DeviceStateModel — one device's local state.
// Contains the device identity and a list of ledgers.

use crate::ledger::spec::*;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Spec types
// ---------------------------------------------------------------------------

pub struct DeviceStateSpec {
    pub device_id: Seq<u8>,
    pub ledgers: Seq<LedgerStateSpec>,
}

// ---------------------------------------------------------------------------
// Helper predicates
// ---------------------------------------------------------------------------

pub open spec fn has_ledger(ledgers: Seq<LedgerStateSpec>, ledger_id: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < ledgers.len() && #[trigger] ledgers[i].ledger_id == ledger_id
}

pub open spec fn find_ledger(ledgers: Seq<LedgerStateSpec>, ledger_id: Seq<u8>) -> int
    recommends has_ledger(ledgers, ledger_id),
{
    choose|i: int| 0 <= i < ledgers.len() && ledgers[i].ledger_id == ledger_id
}

pub open spec fn ledger_ids_unique(ledgers: Seq<LedgerStateSpec>) -> bool {
    forall|i: int, j: int|
        0 <= i < ledgers.len() && 0 <= j < ledgers.len() && i != j
        ==> #[trigger] ledgers[i].ledger_id != #[trigger] ledgers[j].ledger_id
}

// ---------------------------------------------------------------------------
// State machine invariant
// ---------------------------------------------------------------------------

/// Every ledger on this device satisfies its own invariant,
/// and ledger IDs are unique across the device.
pub open spec fn device_invariant(device: DeviceStateSpec) -> bool {
    &&& ledger_ids_unique(device.ledgers)
    &&& forall|i: int| 0 <= i < device.ledgers.len() ==>
        ledger_invariant(#[trigger] device.ledgers[i])
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

/// Create a new empty ledger on this device.
pub open spec fn create_ledger(
    pre: DeviceStateSpec,
    post: DeviceStateSpec,
    ledger_id: Seq<u8>,
    name: Seq<u8>,
    currency: Seq<u8>,
    created_at: i64,
) -> bool {
    &&& !has_ledger(pre.ledgers, ledger_id)
    &&& {
        let new_ledger = LedgerStateSpec {
            ledger_id: ledger_id,
            schema_version: 1,
            name: name,
            currency: currency,
            created_at: created_at,
            users: Seq::empty(),
            bills: Seq::empty(),
            devices: Seq::empty(),
        };
        post == DeviceStateSpec {
            ledgers: pre.ledgers.push(new_ledger),
            ..pre
        }
    }
}

/// Perform an add_user on a specific ledger.
pub open spec fn device_add_user(
    pre: DeviceStateSpec, post: DeviceStateSpec,
    ledger_id: Seq<u8>, user: UserSpec,
) -> bool {
    &&& has_ledger(pre.ledgers, ledger_id)
    &&& {
        let idx = find_ledger(pre.ledgers, ledger_id);
        let pre_ledger = pre.ledgers[idx];
        &&& add_user(pre_ledger, LedgerStateSpec { users: pre_ledger.users.push(user), ..pre_ledger }, user)
        &&& post == DeviceStateSpec {
            ledgers: pre.ledgers.update(idx, LedgerStateSpec { users: pre_ledger.users.push(user), ..pre_ledger }),
            ..pre
        }
    }
}

/// Perform an add_device on a specific ledger.
pub open spec fn device_add_device(
    pre: DeviceStateSpec, post: DeviceStateSpec,
    ledger_id: Seq<u8>, device: DeviceSpec,
) -> bool {
    &&& has_ledger(pre.ledgers, ledger_id)
    &&& {
        let idx = find_ledger(pre.ledgers, ledger_id);
        let pre_ledger = pre.ledgers[idx];
        &&& add_device(pre_ledger, LedgerStateSpec { devices: pre_ledger.devices.push(device), ..pre_ledger }, device)
        &&& post == DeviceStateSpec {
            ledgers: pre.ledgers.update(idx, LedgerStateSpec { devices: pre_ledger.devices.push(device), ..pre_ledger }),
            ..pre
        }
    }
}

/// Perform an add_bill on a specific ledger.
pub open spec fn device_add_bill(
    pre: DeviceStateSpec, post: DeviceStateSpec,
    ledger_id: Seq<u8>, bill: BillSpec,
) -> bool {
    &&& has_ledger(pre.ledgers, ledger_id)
    &&& {
        let idx = find_ledger(pre.ledgers, ledger_id);
        let pre_ledger = pre.ledgers[idx];
        &&& add_bill(pre_ledger, LedgerStateSpec { bills: pre_ledger.bills.push(bill), ..pre_ledger }, bill)
        &&& post == DeviceStateSpec {
            ledgers: pre.ledgers.update(idx, LedgerStateSpec { bills: pre_ledger.bills.push(bill), ..pre_ledger }),
            ..pre
        }
    }
}

}
