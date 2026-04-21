// Low-level Automerge operations used by LedgerDoc.
//
// Each function takes `&mut AutoCommit` (for writes) or `&AutoCommit` (for
// reads) directly so the logic stays testable without a full LedgerDoc wrapper.

use automerge::AutoCommit;
use autosurgeon::{hydrate, reconcile};

use crate::error::UnbillError;
use crate::model::{
    Bill, Currency, Device, EffectiveBills, Ledger, NewBill, NewDevice, NewUser, NodeId, Timestamp,
    Ulid, User,
};

type Result<T> = std::result::Result<T, UnbillError>;

const CURRENT_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Init / read
// ---------------------------------------------------------------------------

/// Write an empty `Ledger` to a freshly created document.
///
/// Must be called exactly once on a new `AutoCommit`. Calling it on a document
/// that already contains a ledger will overwrite the existing data.
pub(super) fn init_ledger(
    doc: &mut AutoCommit,
    ledger_id: Ulid,
    name: String,
    currency: Currency,
    created_at: Timestamp,
) -> Result<()> {
    let ledger = Ledger {
        ledger_id,
        schema_version: CURRENT_SCHEMA_VERSION,
        name,
        currency,
        created_at,
        users: vec![],
        bills: vec![],
        devices: vec![],
    };
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))
}

/// Hydrate the full `Ledger` from the document.
pub(super) fn get_ledger(doc: &AutoCommit) -> Result<Ledger> {
    hydrate(doc).map_err(|e| UnbillError::Other(e.into()))
}

// ---------------------------------------------------------------------------
// Bills
// ---------------------------------------------------------------------------

/// Append a new `Bill` to the ledger. Returns the new bill's ID.
///
/// Returns `UserNotInLedger` if the payer or any user in the share list is not
/// a user in the ledger.
pub(super) fn add_bill(
    doc: &mut AutoCommit,
    input: NewBill,
    created_by_device: NodeId,
    now: Timestamp,
) -> Result<Ulid> {
    let mut ledger = get_ledger(doc)?;

    let user_ids: std::collections::HashSet<Ulid> =
        ledger.users.iter().map(|user| user.user_id).collect();

    let all_users = input
        .payers
        .iter()
        .chain(input.payees.iter())
        .map(|s| &s.user_id);
    for user_id in all_users {
        if !user_ids.contains(user_id) {
            return Err(UnbillError::UserNotInLedger(user_id.to_string()));
        }
    }

    for prev_id in &input.prev {
        if !ledger.bills.iter().any(|b| &b.id == prev_id) {
            return Err(UnbillError::BillNotFound(prev_id.to_string()));
        }
    }

    let bill_id = Ulid::new();
    ledger.bills.push(Bill {
        id: bill_id,
        amount_cents: input.amount_cents,
        description: input.description,
        payers: input.payers,
        payees: input.payees,
        prev: input.prev,
        created_at: now,
        created_by_device,
    });
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))?;
    Ok(bill_id)
}

/// Return all bills in insertion order, including superseded ones.
pub(super) fn list_all_bills(doc: &AutoCommit) -> Result<Vec<Bill>> {
    let ledger = get_ledger(doc)?;
    Ok(ledger.bills)
}

/// Return only effective bills — those whose ID is not referenced in any other
/// bill's `prev`. The order matches insertion order.
pub(super) fn list_bills(doc: &AutoCommit) -> Result<EffectiveBills> {
    let ledger = get_ledger(doc)?;
    let superseded: std::collections::HashSet<Ulid> = ledger
        .bills
        .iter()
        .flat_map(|b| b.prev.iter().copied())
        .collect();
    Ok(EffectiveBills(
        ledger
            .bills
            .into_iter()
            .filter(|b| !superseded.contains(&b.id))
            .collect(),
    ))
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

/// Add a new user to the ledger.
///
/// If the `user_id` already exists this is a no-op.
pub(super) fn add_user(doc: &mut AutoCommit, input: NewUser, now: Timestamp) -> Result<()> {
    let mut ledger = get_ledger(doc)?;
    if ledger
        .users
        .iter()
        .any(|user| user.user_id == input.user_id)
    {
        return Ok(());
    }
    ledger.users.push(User {
        user_id: input.user_id,
        display_name: input.display_name,
        added_at: now,
    });
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))
}

/// Return all users.
pub(super) fn list_users(doc: &AutoCommit) -> Result<Vec<User>> {
    let ledger = get_ledger(doc)?;
    Ok(ledger.users)
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

/// Add a device to the ledger.
///
/// If the NodeId is already present this is a no-op.
pub(super) fn add_device(doc: &mut AutoCommit, input: NewDevice, now: Timestamp) -> Result<()> {
    let mut ledger = get_ledger(doc)?;
    if ledger.devices.iter().any(|d| d.node_id == input.node_id) {
        return Ok(());
    }
    ledger.devices.push(Device {
        node_id: input.node_id,
        added_at: now,
    });
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))
}

/// Return all devices.
pub(super) fn list_devices(doc: &AutoCommit) -> Result<Vec<Device>> {
    let ledger = get_ledger(doc)?;
    Ok(ledger.devices)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Currency, Share};

    fn device() -> NodeId {
        NodeId::from_seed(1)
    }

    fn ts(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms)
    }

    fn uid(n: u128) -> Ulid {
        Ulid::from_u128(n)
    }

    fn fresh_doc() -> AutoCommit {
        let mut doc = AutoCommit::new();
        init_ledger(
            &mut doc,
            uid(1),
            "Test Ledger".into(),
            Currency::from_code("USD").unwrap(),
            ts(0),
        )
        .unwrap();
        doc
    }

    fn doc_with_users(user_ids: &[Ulid]) -> AutoCommit {
        use crate::model::User;
        let mut doc = fresh_doc();
        let mut ledger = get_ledger(&doc).unwrap();
        for &user_id in user_ids {
            ledger.users.push(User {
                user_id,
                display_name: user_id.to_string(),
                added_at: ts(0),
            });
        }
        reconcile(&mut doc, &ledger).unwrap();
        doc
    }

    fn simple_bill(payer: Ulid, payee_users: &[Ulid], amount_cents: i64) -> NewBill {
        NewBill {
            amount_cents,
            description: "Dinner".into(),
            payers: vec![Share {
                user_id: payer,
                shares: 1,
            }],
            payees: payee_users
                .iter()
                .map(|&u| Share {
                    user_id: u,
                    shares: 1,
                })
                .collect(),
            prev: vec![],
        }
    }

    // --- init / get ---

    #[test]
    fn test_init_and_get_ledger_round_trips_metadata() {
        let doc = fresh_doc();
        let ledger = get_ledger(&doc).unwrap();
        assert_eq!(ledger.name, "Test Ledger");
        assert_eq!(ledger.currency.code(), "USD");
        assert_eq!(ledger.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(ledger.bills.is_empty());
        assert!(ledger.users.is_empty());
    }

    // --- add_bill ---

    #[test]
    fn test_add_bill_appears_in_list_bills() {
        let alice = uid(1);
        let bob = uid(2);
        let mut doc = doc_with_users(&[alice, bob]);
        let bill_id = add_bill(
            &mut doc,
            simple_bill(alice, &[alice, bob], 3000),
            device(),
            ts(1),
        )
        .unwrap();
        let bills = list_bills(&doc).unwrap();
        assert_eq!(bills.0.len(), 1);
        assert_eq!(bills.0[0].id, bill_id);
        assert_eq!(bills.0[0].amount_cents, 3000);
    }

    #[test]
    fn test_add_multiple_bills_preserves_order() {
        let alice = uid(1);
        let mut doc = doc_with_users(&[alice]);
        let id1 = add_bill(
            &mut doc,
            simple_bill(alice, &[alice], 1000),
            device(),
            ts(1),
        )
        .unwrap();
        let id2 = add_bill(
            &mut doc,
            simple_bill(alice, &[alice], 2000),
            device(),
            ts(2),
        )
        .unwrap();
        let bills = list_bills(&doc).unwrap();
        assert_eq!(bills.0[0].id, id1);
        assert_eq!(bills.0[1].id, id2);
    }

    #[test]
    fn test_add_bill_rejects_non_user_payer() {
        let alice = uid(1);
        let stranger = uid(99);
        let mut doc = doc_with_users(&[alice]);
        let result = add_bill(
            &mut doc,
            simple_bill(stranger, &[alice], 1000),
            device(),
            ts(1),
        );
        assert!(
            matches!(result, Err(UnbillError::UserNotInLedger(_))),
            "expected UserNotInLedger, got {result:?}"
        );
    }

    #[test]
    fn test_add_bill_rejects_non_user_share_user() {
        let alice = uid(1);
        let stranger = uid(99);
        let mut doc = doc_with_users(&[alice]);
        let result = add_bill(
            &mut doc,
            simple_bill(alice, &[alice, stranger], 1000),
            device(),
            ts(1),
        );
        assert!(
            matches!(result, Err(UnbillError::UserNotInLedger(_))),
            "expected UserNotInLedger, got {result:?}"
        );
    }

    // --- amendment via prev ---

    #[test]
    fn test_amendment_supersedes_original() {
        let alice = uid(1);
        let mut doc = doc_with_users(&[alice]);
        let original_id = add_bill(
            &mut doc,
            simple_bill(alice, &[alice], 1000),
            device(),
            ts(1),
        )
        .unwrap();
        add_bill(
            &mut doc,
            NewBill {
                prev: vec![original_id],
                ..simple_bill(alice, &[alice], 2000)
            },
            device(),
            ts(2),
        )
        .unwrap();
        let bills = list_bills(&doc).unwrap();
        assert_eq!(bills.0.len(), 1, "original should be superseded");
        assert_eq!(bills.0[0].amount_cents, 2000);
    }

    #[test]
    fn test_amendment_with_unknown_prev_returns_error() {
        let alice = uid(1);
        let mut doc = doc_with_users(&[alice]);
        let result = add_bill(
            &mut doc,
            NewBill {
                prev: vec![uid(999)],
                ..simple_bill(alice, &[alice], 1000)
            },
            device(),
            ts(0),
        );
        assert!(matches!(result, Err(UnbillError::BillNotFound(_))));
    }

    // --- list_users ---

    #[test]
    fn test_list_users_returns_all() {
        let mut doc = fresh_doc();
        add_user(
            &mut doc,
            NewUser {
                user_id: uid(1),
                display_name: "Alice".into(),
            },
            ts(0),
        )
        .unwrap();
        add_user(
            &mut doc,
            NewUser {
                user_id: uid(2),
                display_name: "Bob".into(),
            },
            ts(1),
        )
        .unwrap();
        let users = list_users(&doc).unwrap();
        assert_eq!(users.len(), 2);
    }

    // --- save/load round-trip ---

    #[test]
    fn test_save_and_reload_preserves_bills() {
        let alice = uid(1);
        let bob = uid(2);
        let mut doc = doc_with_users(&[alice, bob]);
        let bill_id = add_bill(
            &mut doc,
            simple_bill(alice, &[alice, bob], 6000),
            device(),
            ts(1),
        )
        .unwrap();

        let bytes = doc.save();
        let reloaded = AutoCommit::load(&bytes).unwrap();
        let bills = list_bills(&reloaded).unwrap();
        assert_eq!(bills.0.len(), 1);
        assert_eq!(bills.0[0].id, bill_id);
        assert_eq!(bills.0[0].amount_cents, 6000);
    }

    // --- devices ---

    fn dev(seed: u8) -> NodeId {
        NodeId::from_seed(seed)
    }

    fn new_device(seed: u8) -> NewDevice {
        NewDevice { node_id: dev(seed) }
    }

    #[test]
    fn test_add_device_appears_in_list() {
        let mut doc = fresh_doc();
        add_device(&mut doc, new_device(1), ts(0)).unwrap();
        let devices = list_devices(&doc).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].node_id, dev(1));
        assert_eq!(devices[0].added_at, ts(0));
    }

    #[test]
    fn test_add_device_duplicate_is_noop() {
        let mut doc = fresh_doc();
        add_device(&mut doc, new_device(1), ts(0)).unwrap();
        add_device(&mut doc, new_device(1), ts(1)).unwrap();
        assert_eq!(list_devices(&doc).unwrap().len(), 1);
    }
}
