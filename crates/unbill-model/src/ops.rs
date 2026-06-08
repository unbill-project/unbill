// Low-level Automerge operations used by LedgerDoc.
//
// Each function takes `&mut AutoCommit` (for writes) or `&AutoCommit` (for
// reads) directly so the logic stays testable without a full LedgerDoc wrapper.

use automerge::AutoCommit;
use autosurgeon::{hydrate, reconcile};

use unbill_model_verified::ledger::exec as v;

use crate::verified_bridge::{ledger_to_model, model_to_ledger};
use crate::{
    Bill, BillId, Currency, Device, EffectiveBills, Ledger, LedgerId, NewBill, NewDevice, NewUser,
    NodeId, Timestamp, UnbillError, User, UserId,
};

type Result<T> = std::result::Result<T, UnbillError>;

#[cfg(test)]
const CURRENT_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Init / read
// ---------------------------------------------------------------------------

/// Write an empty `Ledger` to a freshly created document.
pub(super) fn init_ledger(
    doc: &mut AutoCommit,
    ledger_id: LedgerId,
    name: String,
    currency: Currency,
    created_at: Timestamp,
) -> Result<()> {
    let model = v::exec_init(
        ledger_id.to_u128(),
        name.into_bytes(),
        currency.code().as_bytes().to_vec(),
        created_at.as_millis(),
    );
    let ledger = model_to_ledger(&model).map_err(|e| UnbillError::Reconcile(e.0))?;
    reconcile(doc, &ledger).map_err(|e| UnbillError::Reconcile(e.to_string()))
}

/// Hydrate the full `Ledger` from the document.
pub(super) fn get_ledger(doc: &AutoCommit) -> Result<Ledger> {
    hydrate(doc).map_err(|e| UnbillError::Reconcile(e.to_string()))
}

// ---------------------------------------------------------------------------
// Bills
// ---------------------------------------------------------------------------

pub(super) fn add_bill(
    doc: &mut AutoCommit,
    input: NewBill,
    created_by_device: NodeId,
    now: Timestamp,
) -> Result<BillId> {
    let ledger = get_ledger(doc)?;

    // Validation — same checks as before.
    let user_ids: std::collections::HashSet<UserId> =
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

    // Mutation through verified model.
    let bill_id = BillId::new();
    let mut model = ledger_to_model(&ledger);
    v::exec_add_bill(
        &mut model,
        v::Bill {
            id: bill_id.to_u128(),
            amount_cents: input.amount_cents,
            description: input.description.into_bytes(),
            payers: input
                .payers
                .iter()
                .map(|s| v::Share {
                    user_id: s.user_id.to_u128(),
                    weight: s.shares,
                })
                .collect(),
            payees: input
                .payees
                .iter()
                .map(|s| v::Share {
                    user_id: s.user_id.to_u128(),
                    weight: s.shares,
                })
                .collect(),
            prev: input.prev.iter().map(|id| id.to_u128()).collect(),
            created_at: now.as_millis(),
            created_by_device: created_by_device.to_string().into_bytes(),
        },
    );
    let ledger = model_to_ledger(&model).map_err(|e| UnbillError::Reconcile(e.0))?;
    reconcile(doc, &ledger).map_err(|e| UnbillError::Reconcile(e.to_string()))?;
    Ok(bill_id)
}

/// Return all bills in insertion order, including superseded ones.
pub(super) fn list_all_bills(doc: &AutoCommit) -> Result<Vec<Bill>> {
    let ledger = get_ledger(doc)?;
    Ok(ledger.bills)
}

/// Return only effective bills — those whose ID is not referenced in any other
/// bill's `prev`.
pub(super) fn list_bills(doc: &AutoCommit) -> Result<EffectiveBills> {
    let ledger = get_ledger(doc)?;
    let superseded: std::collections::HashSet<BillId> = ledger
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

pub(super) fn add_user(doc: &mut AutoCommit, input: NewUser, now: Timestamp) -> Result<()> {
    let ledger = get_ledger(doc)?;
    if ledger
        .users
        .iter()
        .any(|user| user.user_id == input.user_id)
    {
        return Ok(());
    }
    let mut model = ledger_to_model(&ledger);
    v::exec_add_user(
        &mut model,
        v::User {
            user_id: input.user_id.to_u128(),
            display_name: input.display_name.into_bytes(),
            added_at: now.as_millis(),
        },
    );
    let ledger = model_to_ledger(&model).map_err(|e| UnbillError::Reconcile(e.0))?;
    reconcile(doc, &ledger).map_err(|e| UnbillError::Reconcile(e.to_string()))
}

pub(super) fn list_users(doc: &AutoCommit) -> Result<Vec<User>> {
    let ledger = get_ledger(doc)?;
    Ok(ledger.users)
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

pub(super) fn add_device(doc: &mut AutoCommit, input: NewDevice, now: Timestamp) -> Result<()> {
    let ledger = get_ledger(doc)?;
    if ledger.devices.iter().any(|d| d.node_id == input.node_id) {
        return Ok(());
    }
    let mut model = ledger_to_model(&ledger);
    v::exec_add_device(
        &mut model,
        v::Device {
            node_id: input.node_id.to_string().into_bytes(),
            added_at: now.as_millis(),
        },
    );
    let ledger = model_to_ledger(&model).map_err(|e| UnbillError::Reconcile(e.0))?;
    reconcile(doc, &ledger).map_err(|e| UnbillError::Reconcile(e.to_string()))
}

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
    use crate::{Currency, Share};

    fn device() -> NodeId {
        NodeId::from_seed(1)
    }

    fn ts(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms)
    }

    fn uid(n: u128) -> UserId {
        UserId::from_u128(n)
    }

    fn lid(n: u128) -> LedgerId {
        LedgerId::from_u128(n)
    }

    fn fresh_doc() -> AutoCommit {
        let mut doc = AutoCommit::new();
        init_ledger(
            &mut doc,
            lid(1),
            "Test Ledger".into(),
            Currency::from_code("USD").unwrap(),
            ts(0),
        )
        .unwrap();
        doc
    }

    fn doc_with_users(user_ids: &[UserId]) -> AutoCommit {
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

    fn simple_bill(payer: UserId, payee_users: &[UserId], amount_cents: i64) -> NewBill {
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
    fn test_add_device_appears_in_list() {
        let mut doc = fresh_doc();
        add_device(
            &mut doc,
            NewDevice {
                node_id: NodeId::from_seed(1),
            },
            ts(0),
        )
        .unwrap();
        let devices = list_devices(&doc).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].node_id, NodeId::from_seed(1));
    }

    #[test]
    fn test_add_device_duplicate_is_noop() {
        let mut doc = fresh_doc();
        add_device(
            &mut doc,
            NewDevice {
                node_id: NodeId::from_seed(1),
            },
            ts(0),
        )
        .unwrap();
        add_device(
            &mut doc,
            NewDevice {
                node_id: NodeId::from_seed(1),
            },
            ts(1),
        )
        .unwrap();
        assert_eq!(list_devices(&doc).unwrap().len(), 1);
    }
}
