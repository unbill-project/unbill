// Checker functions: runtime validation that implies exec preconditions.
// Each checker is proven to only return Ok when the corresponding exec requires holds.
// Limits are tighter than the mathematical spec bounds for user-friendly errors.

use super::exec::*;
use super::spec;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// User-friendly limits
// ---------------------------------------------------------------------------

pub const MAX_USERS: usize = 2_000_000;
pub const MAX_DEVICES: usize = 2_000_000;
pub const MAX_BILLS: usize = 2_000_000;
pub const MAX_AMOUNT_CENTS: i64 = 2_000_000_000; // ~$20M
pub const MAX_SHARES_PER_SIDE: usize = 1_000;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[cfg_attr(feature = "thiserror", derive(Debug, thiserror::Error))]
pub enum AddUserError {
    #[cfg_attr(feature = "thiserror", error("duplicate user"))]
    DuplicateUser,
    #[cfg_attr(feature = "thiserror", error("too many users"))]
    TooManyUsers,
}

#[cfg_attr(feature = "thiserror", derive(Debug, thiserror::Error))]
pub enum AddDeviceError {
    #[cfg_attr(feature = "thiserror", error("duplicate device"))]
    DuplicateDevice,
    #[cfg_attr(feature = "thiserror", error("too many devices"))]
    TooManyDevices,
}

#[cfg_attr(feature = "thiserror", derive(Debug, thiserror::Error))]
pub enum AddBillError {
    #[cfg_attr(feature = "thiserror", error("negative amount"))]
    NegativeAmount,
    #[cfg_attr(feature = "thiserror", error("amount too large"))]
    AmountTooLarge,
    #[cfg_attr(feature = "thiserror", error("no payers"))]
    NoPayers,
    #[cfg_attr(feature = "thiserror", error("no payees"))]
    NoPayees,
    #[cfg_attr(feature = "thiserror", error("too many payers"))]
    TooManyPayers,
    #[cfg_attr(feature = "thiserror", error("too many payees"))]
    TooManyPayees,
    #[cfg_attr(feature = "thiserror", error("payer at index {index} has zero weight"))]
    ZeroPayerWeight { index: usize },
    #[cfg_attr(feature = "thiserror", error("payee at index {index} has zero weight"))]
    ZeroPayeeWeight { index: usize },
    #[cfg_attr(feature = "thiserror", error("payer at index {index} not in ledger"))]
    PayerNotInLedger { index: usize },
    #[cfg_attr(feature = "thiserror", error("payee at index {index} not in ledger"))]
    PayeeNotInLedger { index: usize },
    #[cfg_attr(feature = "thiserror", error("device not in ledger"))]
    DeviceNotInLedger,
    #[cfg_attr(feature = "thiserror", error("prev bill at index {index} not found"))]
    PrevBillNotFound { index: usize },
    #[cfg_attr(feature = "thiserror", error("duplicate bill ID"))]
    DuplicateBillId,
    #[cfg_attr(feature = "thiserror", error("too many bills"))]
    TooManyBills,
}

// ---------------------------------------------------------------------------
// Proof lemmas for total_weight
// ---------------------------------------------------------------------------

proof fn total_weight_all_positive(shares: Seq<spec::ShareSpec>)
    requires
        shares.len() > 0,
        forall|i: int| 0 <= i < shares.len() ==> (#[trigger] shares[i]).weight > 0u32,
    ensures
        spec::total_weight(shares) > 0,
    decreases shares.len(),
{
    if shares.len() > 1 {
        assert forall|i: int| 0 <= i < shares.drop_last().len()
            implies (#[trigger] shares.drop_last()[i]).weight > 0u32
        by { assert(shares.drop_last()[i] == shares[i]); }
        total_weight_all_positive(shares.drop_last());
    } else {
        super::proof::total_weight_includes_each(shares, 0);
    }
}

proof fn total_weight_upper_bound(shares: Seq<spec::ShareSpec>)
    ensures spec::total_weight(shares) <= shares.len() as int * u32::MAX as int,
    decreases shares.len(),
{
    if shares.len() > 0 {
        total_weight_upper_bound(shares.drop_last());
    }
}

// ---------------------------------------------------------------------------
// Containment helpers (ensures use ledger@ directly, no Seq::map)
// ---------------------------------------------------------------------------

fn user_id_in_ledger(ledger: &LedgerState, user_id: u128) -> (result: bool)
    ensures result == spec::has_user(ledger@.users, user_id),
{
    let mut i: usize = 0;
    while i < ledger.users.len()
        invariant
            i <= ledger.users.len(),
            forall|j: int| 0 <= j < i as int ==>
                (#[trigger] ledger@.users[j]).user_id != user_id,
        decreases ledger.users.len() - i,
    {
        proof { ledger_user_at(*ledger, i as int); }
        if ledger.users[i].user_id == user_id {
            proof { assert(ledger@.users[i as int].user_id == user_id); }
            return true;
        }
        i += 1;
    }
    false
}

fn device_in_ledger(ledger: &LedgerState, node_id: &Vec<u8>) -> (result: bool)
    ensures result == spec::has_device(ledger@.devices, node_id@),
{
    let mut i: usize = 0;
    while i < ledger.devices.len()
        invariant
            i <= ledger.devices.len(),
            forall|j: int| 0 <= j < i as int ==>
                (#[trigger] ledger@.devices[j]).node_id != node_id@,
        decreases ledger.devices.len() - i,
    {
        proof { ledger_device_at(*ledger, i as int); }
        if vec_u8_eq(&ledger.devices[i].node_id, node_id) {
            proof { assert(ledger@.devices[i as int].node_id == node_id@); }
            return true;
        }
        i += 1;
    }
    false
}

fn bill_id_in_ledger(ledger: &LedgerState, bill_id: u128) -> (result: bool)
    ensures result == spec::has_bill(ledger@.bills, bill_id),
{
    let mut i: usize = 0;
    while i < ledger.bills.len()
        invariant
            i <= ledger.bills.len(),
            forall|j: int| 0 <= j < i as int ==>
                (#[trigger] ledger@.bills[j]).id != bill_id,
        decreases ledger.bills.len() - i,
    {
        proof { ledger_bill_at(*ledger, i as int); }
        if ledger.bills[i].id == bill_id {
            proof { assert(ledger@.bills[i as int].id == bill_id); }
            return true;
        }
        i += 1;
    }
    false
}

fn vec_u8_eq(a: &Vec<u8>, b: &Vec<u8>) -> (result: bool)
    ensures result == (a@ == b@),
{
    if a.len() != b.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.len()
        invariant
            i <= a.len(),
            a@.len() == b@.len(),
            forall|j: int| 0 <= j < i as int ==> a@[j] == b@[j],
        decreases a.len() - i,
    {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    proof { assert(a@ =~= b@); }
    true
}

// ---------------------------------------------------------------------------
// Checkers
// ---------------------------------------------------------------------------

/// exec_init has no preconditions — always succeeds.
pub fn check_init() -> (result: bool)
    ensures result == true,
{
    true
}

pub fn check_add_user(ledger: &LedgerState, user: &User) -> (result: Result<(), AddUserError>)
    requires spec::ledger_invariant(ledger@),
    ensures result.is_ok() ==> spec::exec_add_user_requires(ledger@, user@),
{
    if ledger.users.len() >= MAX_USERS {
        return Err(AddUserError::TooManyUsers);
    }
    if user_id_in_ledger(ledger, user.user_id) {
        return Err(AddUserError::DuplicateUser);
    }
    Ok(())
}

pub fn check_add_device(ledger: &LedgerState, device: &Device) -> (result: Result<(), AddDeviceError>)
    requires spec::ledger_invariant(ledger@),
    ensures result.is_ok() ==> spec::exec_add_device_requires(ledger@, device@),
{
    if ledger.devices.len() >= MAX_DEVICES {
        return Err(AddDeviceError::TooManyDevices);
    }
    if device_in_ledger(ledger, &device.node_id) {
        return Err(AddDeviceError::DuplicateDevice);
    }
    Ok(())
}

pub fn check_add_bill(ledger: &LedgerState, bill: &Bill) -> (result: Result<(), AddBillError>)
    requires spec::ledger_invariant(ledger@),
    ensures result.is_ok() ==> spec::exec_add_bill_requires(ledger@, bill@),
{
    // --- Amount ---
    if bill.amount_cents < 0 {
        return Err(AddBillError::NegativeAmount);
    }
    if bill.amount_cents > MAX_AMOUNT_CENTS {
        return Err(AddBillError::AmountTooLarge);
    }

    // --- Share structure ---
    if bill.payers.len() == 0 {
        return Err(AddBillError::NoPayers);
    }
    if bill.payees.len() == 0 {
        return Err(AddBillError::NoPayees);
    }
    if bill.payers.len() > MAX_SHARES_PER_SIDE {
        return Err(AddBillError::TooManyPayers);
    }
    if bill.payees.len() > MAX_SHARES_PER_SIDE {
        return Err(AddBillError::TooManyPayees);
    }

    // --- Payer weights ---
    let mut i: usize = 0;
    while i < bill.payers.len()
        invariant
            i <= bill.payers.len(),
            forall|j: int| 0 <= j < i as int ==>
                (#[trigger] bill.payers@[j]).weight > 0u32,
        decreases bill.payers.len() - i,
    {
        if bill.payers[i].weight == 0 {
            return Err(AddBillError::ZeroPayerWeight { index: i });
        }
        i += 1;
    }

    // --- Payee weights ---
    let mut i: usize = 0;
    while i < bill.payees.len()
        invariant
            i <= bill.payees.len(),
            forall|j: int| 0 <= j < i as int ==>
                (#[trigger] bill.payees@[j]).weight > 0u32,
        decreases bill.payees.len() - i,
    {
        if bill.payees[i].weight == 0 {
            return Err(AddBillError::ZeroPayeeWeight { index: i });
        }
        i += 1;
    }

    // --- Payers reference known users ---
    let mut i: usize = 0;
    while i < bill.payers.len()
        invariant
            i <= bill.payers.len(),
            forall|j: int| 0 <= j < i as int ==>
                spec::has_user(ledger@.users, (#[trigger] bill.payers@[j]).user_id),
        decreases bill.payers.len() - i,
    {
        if !user_id_in_ledger(ledger, bill.payers[i].user_id) {
            return Err(AddBillError::PayerNotInLedger { index: i });
        }
        i += 1;
    }

    // --- Payees reference known users ---
    let mut i: usize = 0;
    while i < bill.payees.len()
        invariant
            i <= bill.payees.len(),
            forall|j: int| 0 <= j < i as int ==>
                spec::has_user(ledger@.users, (#[trigger] bill.payees@[j]).user_id),
        decreases bill.payees.len() - i,
    {
        if !user_id_in_ledger(ledger, bill.payees[i].user_id) {
            return Err(AddBillError::PayeeNotInLedger { index: i });
        }
        i += 1;
    }

    // --- Device exists ---
    if !device_in_ledger(ledger, &bill.created_by_device) {
        return Err(AddBillError::DeviceNotInLedger);
    }

    // --- Prev bills exist ---
    let mut i: usize = 0;
    while i < bill.prev.len()
        invariant
            i <= bill.prev.len(),
            forall|j: int| 0 <= j < i as int ==>
                spec::has_bill(ledger@.bills, #[trigger] bill.prev@[j]),
        decreases bill.prev.len() - i,
    {
        if !bill_id_in_ledger(ledger, bill.prev[i]) {
            return Err(AddBillError::PrevBillNotFound { index: i });
        }
        i += 1;
    }

    // --- Bill ID unique ---
    if bill_id_in_ledger(ledger, bill.id) {
        return Err(AddBillError::DuplicateBillId);
    }

    // --- Bill count ---
    if ledger.bills.len() >= MAX_BILLS {
        return Err(AddBillError::TooManyBills);
    }

    // --- Proof: tighter checks imply spec requires ---
    proof {
        bill_bridge(*bill);

        // Bridge payer weights: bill@.payers[j].weight == bill.payers@[j].weight
        assert forall|j: int| 0 <= j < bill@.payers.len()
            implies (#[trigger] bill@.payers[j]).weight > 0u32
        by {
            assert(bill@.payers[j] == shares_to_specs(bill.payers@)[j]);
        }
        total_weight_all_positive(bill@.payers);
        total_weight_upper_bound(bill@.payers);

        // Bridge payee weights
        assert forall|j: int| 0 <= j < bill@.payees.len()
            implies (#[trigger] bill@.payees[j]).weight > 0u32
        by {
            assert(bill@.payees[j] == shares_to_specs(bill.payees@)[j]);
        }
        total_weight_all_positive(bill@.payees);
        total_weight_upper_bound(bill@.payees);

        // Bridge shares_reference_known_users for payers
        assert forall|j: int| 0 <= j < bill@.payers.len()
            implies spec::has_user(ledger@.users, (#[trigger] bill@.payers[j]).user_id)
        by {
            assert(bill@.payers[j] == shares_to_specs(bill.payers@)[j]);
        }

        // Bridge shares_reference_known_users for payees
        assert forall|j: int| 0 <= j < bill@.payees.len()
            implies spec::has_user(ledger@.users, (#[trigger] bill@.payees[j]).user_id)
        by {
            assert(bill@.payees[j] == shares_to_specs(bill.payees@)[j]);
        }

        // Bridge prev bills
        assert forall|j: int| 0 <= j < bill@.prev.len()
            implies spec::has_bill(ledger@.bills, #[trigger] bill@.prev[j])
        by {
            assert(bill@.prev[j] == bill.prev@[j]);
        }

        // Assemble add_bill
        let post = spec::LedgerStateSpec {
            bills: ledger@.bills.push(bill@),
            ..ledger@
        };
        assert(spec::add_bill(ledger@, post, bill@));
    }

    Ok(())
}

}
