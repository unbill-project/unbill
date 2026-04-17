// Low-level Automerge operations used by LedgerDoc.
//
// Each function takes `&mut AutoCommit` (for writes) or `&AutoCommit` (for
// reads) directly so the logic stays testable without a full LedgerDoc wrapper.

use automerge::AutoCommit;
use autosurgeon::{hydrate, reconcile};

use crate::error::UnbillError;
use crate::model::{
    Amendment, Bill, BillAmendment, Currency, EffectiveBill, Ledger, Member, NewBill, NewMember,
    NodeId, Timestamp, Ulid,
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
        members: vec![],
        bills: vec![],
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
/// Returns `UserNotMember` if the payer or any share participant is not an
/// active (non-removed) member of the ledger.
pub(super) fn add_bill(
    doc: &mut AutoCommit,
    input: NewBill,
    created_by_device: NodeId,
    now: Timestamp,
) -> Result<Ulid> {
    let mut ledger = get_ledger(doc)?;

    let member_ids: std::collections::HashSet<Ulid> = ledger
        .members
        .iter()
        .filter(|m| !m.removed)
        .map(|m| m.user_id)
        .collect();

    let all_users = std::iter::once(&input.payer_user_id)
        .chain(input.shares.iter().map(|s| &s.user_id));
    for user_id in all_users {
        if !member_ids.contains(user_id) {
            return Err(UnbillError::UserNotMember(user_id.to_string()));
        }
    }

    let bill_id = Ulid::new();
    ledger.bills.push(Bill {
        id: bill_id,
        payer_user_id: input.payer_user_id,
        amount_cents: input.amount_cents,
        description: input.description,
        shares: input.shares,
        created_at: now,
        created_by_device,
        deleted: false,
        amendments: vec![],
    });
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))?;
    Ok(bill_id)
}

/// Append an `Amendment` to an existing bill.
pub(super) fn amend_bill(
    doc: &mut AutoCommit,
    bill_id: &Ulid,
    input: BillAmendment,
    now: Timestamp,
) -> Result<()> {
    let mut ledger = get_ledger(doc)?;
    let bill = ledger
        .bills
        .iter_mut()
        .find(|b| &b.id == bill_id)
        .ok_or_else(|| UnbillError::BillNotFound(bill_id.to_string()))?;
    bill.amendments.push(Amendment {
        id: Ulid::new(),
        new_amount_cents: input.new_amount_cents,
        new_description: input.new_description,
        new_shares: input.new_shares,
        author_user_id: input.author_user_id,
        created_at: now,
        reason: input.reason,
    });
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))
}

/// Tombstone-delete a bill (`deleted = true`).
pub(super) fn delete_bill(doc: &mut AutoCommit, bill_id: &Ulid) -> Result<()> {
    let mut ledger = get_ledger(doc)?;
    let bill = ledger
        .bills
        .iter_mut()
        .find(|b| &b.id == bill_id)
        .ok_or_else(|| UnbillError::BillNotFound(bill_id.to_string()))?;
    bill.deleted = true;
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))
}

/// Restore a tombstoned bill (`deleted = false`).
pub(super) fn restore_bill(doc: &mut AutoCommit, bill_id: &Ulid) -> Result<()> {
    let mut ledger = get_ledger(doc)?;
    let bill = ledger
        .bills
        .iter_mut()
        .find(|b| &b.id == bill_id)
        .ok_or_else(|| UnbillError::BillNotFound(bill_id.to_string()))?;
    bill.deleted = false;
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))
}

/// Project all bills to their effective (post-amendment) view.
pub(super) fn list_bills(doc: &AutoCommit) -> Result<Vec<EffectiveBill>> {
    let ledger = get_ledger(doc)?;
    Ok(ledger.bills.iter().map(EffectiveBill::from).collect())
}

// ---------------------------------------------------------------------------
// Members
// ---------------------------------------------------------------------------

/// Add a new member to the ledger.
///
/// Returns `UserNotMember` variant reused as "already a member" is not an
/// error — if the user_id already exists (and is not removed), this is a no-op.
pub(super) fn add_member(
    doc: &mut AutoCommit,
    input: NewMember,
    now: Timestamp,
) -> Result<()> {
    let mut ledger = get_ledger(doc)?;
    // If already an active member, no-op.
    if ledger
        .members
        .iter()
        .any(|m| m.user_id == input.user_id && !m.removed)
    {
        return Ok(());
    }
    // If previously removed, re-activate.
    if let Some(m) = ledger
        .members
        .iter_mut()
        .find(|m| m.user_id == input.user_id && m.removed)
    {
        m.removed = false;
        return reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()));
    }
    ledger.members.push(Member {
        user_id: input.user_id,
        display_name: input.display_name,
        devices: vec![],
        added_at: now,
        added_by: input.added_by,
        removed: false,
    });
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))
}

/// Remove a member (tombstone: `removed = true`).
pub(super) fn remove_member(doc: &mut AutoCommit, user_id: &Ulid) -> Result<()> {
    let mut ledger = get_ledger(doc)?;
    let member = ledger
        .members
        .iter_mut()
        .find(|m| &m.user_id == user_id && !m.removed)
        .ok_or_else(|| UnbillError::MemberNotFound(user_id.to_string()))?;
    member.removed = true;
    reconcile(doc, &ledger).map_err(|e| UnbillError::Other(e.into()))
}

/// Return all non-removed members.
pub(super) fn list_members(doc: &AutoCommit) -> Result<Vec<Member>> {
    let ledger = get_ledger(doc)?;
    Ok(ledger.members.into_iter().filter(|m| !m.removed).collect())
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

    fn doc_with_members(user_ids: &[Ulid]) -> AutoCommit {
        use crate::model::Member;
        let mut doc = fresh_doc();
        let mut ledger = get_ledger(&doc).unwrap();
        for &user_id in user_ids {
            ledger.members.push(Member {
                user_id,
                display_name: user_id.to_string(),
                devices: vec![],
                added_at: ts(0),
                added_by: uid(0),
                removed: false,
            });
        }
        reconcile(&mut doc, &ledger).unwrap();
        doc
    }

    fn simple_bill(payer: Ulid, participants: &[Ulid], amount_cents: i64) -> NewBill {
        NewBill {
            payer_user_id: payer,
            amount_cents,
            description: "Dinner".into(),
            shares: participants
                .iter()
                .map(|&u| Share {
                    user_id: u,
                    shares: 1,
                })
                .collect(),
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
        assert!(ledger.members.is_empty());
    }

    // --- add_bill ---

    #[test]
    fn test_add_bill_appears_in_list_bills() {
        let alice = uid(1);
        let bob = uid(2);
        let mut doc = doc_with_members(&[alice, bob]);
        let bill_id = add_bill(
            &mut doc,
            simple_bill(alice, &[alice, bob], 3000),
            device(),
            ts(1),
        )
        .unwrap();
        let bills = list_bills(&doc).unwrap();
        assert_eq!(bills.len(), 1);
        assert_eq!(bills[0].id, bill_id);
        assert_eq!(bills[0].amount_cents, 3000);
        assert!(!bills[0].is_deleted);
        assert!(!bills[0].was_amended);
    }

    #[test]
    fn test_add_multiple_bills_preserves_order() {
        let alice = uid(1);
        let mut doc = doc_with_members(&[alice]);
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
        assert_eq!(bills[0].id, id1);
        assert_eq!(bills[1].id, id2);
    }

    #[test]
    fn test_add_bill_rejects_non_member_payer() {
        let alice = uid(1);
        let stranger = uid(99);
        let mut doc = doc_with_members(&[alice]);
        let result = add_bill(
            &mut doc,
            simple_bill(stranger, &[alice], 1000),
            device(),
            ts(1),
        );
        assert!(
            matches!(result, Err(UnbillError::UserNotMember(_))),
            "expected UserNotMember, got {result:?}"
        );
    }

    #[test]
    fn test_add_bill_rejects_non_member_participant() {
        let alice = uid(1);
        let stranger = uid(99);
        let mut doc = doc_with_members(&[alice]);
        let result = add_bill(
            &mut doc,
            simple_bill(alice, &[alice, stranger], 1000),
            device(),
            ts(1),
        );
        assert!(
            matches!(result, Err(UnbillError::UserNotMember(_))),
            "expected UserNotMember, got {result:?}"
        );
    }

    #[test]
    fn test_add_bill_rejects_removed_member() {
        use crate::model::Member;
        let alice = uid(1);
        let bob = uid(2);
        let mut doc = doc_with_members(&[alice]);
        // Add Bob but mark him removed.
        let mut ledger = get_ledger(&doc).unwrap();
        ledger.members.push(Member {
            user_id: bob,
            display_name: "Bob".into(),
            devices: vec![],
            added_at: ts(0),
            added_by: alice,
            removed: true,
        });
        reconcile(&mut doc, &ledger).unwrap();

        let result = add_bill(&mut doc, simple_bill(alice, &[alice, bob], 1000), device(), ts(1));
        assert!(
            matches!(result, Err(UnbillError::UserNotMember(_))),
            "expected UserNotMember for removed member, got {result:?}"
        );
    }

    // --- amend_bill ---

    #[test]
    fn test_amend_bill_updates_effective_view() {
        let alice = uid(1);
        let mut doc = doc_with_members(&[alice]);
        let bill_id = add_bill(
            &mut doc,
            simple_bill(alice, &[alice], 1000),
            device(),
            ts(1),
        )
        .unwrap();
        amend_bill(
            &mut doc,
            &bill_id,
            BillAmendment {
                new_amount_cents: Some(2000),
                new_description: Some("Updated".into()),
                new_shares: None,
                author_user_id: alice,
                reason: None,
            },
            ts(2),
        )
        .unwrap();
        let bills = list_bills(&doc).unwrap();
        assert_eq!(bills[0].amount_cents, 2000);
        assert_eq!(bills[0].description, "Updated");
        assert!(bills[0].was_amended);
        assert_eq!(bills[0].history.len(), 1);
    }

    #[test]
    fn test_amend_unknown_bill_returns_error() {
        let mut doc = fresh_doc();
        let result = amend_bill(
            &mut doc,
            &uid(999),
            BillAmendment {
                new_amount_cents: Some(1),
                new_description: None,
                new_shares: None,
                author_user_id: uid(1),
                reason: None,
            },
            ts(0),
        );
        assert!(matches!(result, Err(UnbillError::BillNotFound(_))));
    }

    // --- delete / restore ---

    #[test]
    fn test_delete_bill_sets_tombstone() {
        let alice = uid(1);
        let mut doc = doc_with_members(&[alice]);
        let bill_id =
            add_bill(&mut doc, simple_bill(alice, &[alice], 500), device(), ts(1)).unwrap();
        delete_bill(&mut doc, &bill_id).unwrap();
        let bills = list_bills(&doc).unwrap();
        assert!(bills[0].is_deleted);
    }

    #[test]
    fn test_restore_bill_clears_tombstone() {
        let alice = uid(1);
        let mut doc = doc_with_members(&[alice]);
        let bill_id =
            add_bill(&mut doc, simple_bill(alice, &[alice], 500), device(), ts(1)).unwrap();
        delete_bill(&mut doc, &bill_id).unwrap();
        restore_bill(&mut doc, &bill_id).unwrap();
        let bills = list_bills(&doc).unwrap();
        assert!(!bills[0].is_deleted);
    }

    #[test]
    fn test_delete_unknown_bill_returns_error() {
        let mut doc = fresh_doc();
        let result = delete_bill(&mut doc, &uid(999));
        assert!(matches!(result, Err(UnbillError::BillNotFound(_))));
    }

    // --- list_members ---

    #[test]
    fn test_list_members_excludes_removed() {
        use crate::model::Member;
        let mut doc = fresh_doc();
        let mut ledger = get_ledger(&doc).unwrap();
        ledger.members.push(Member {
            user_id: uid(1),
            display_name: "Alice".into(),
            devices: vec![],
            added_at: ts(0),
            added_by: uid(1),
            removed: false,
        });
        ledger.members.push(Member {
            user_id: uid(2),
            display_name: "Bob".into(),
            devices: vec![],
            added_at: ts(0),
            added_by: uid(1),
            removed: true,
        });
        reconcile(&mut doc, &ledger).unwrap();

        let members = list_members(&doc).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user_id, uid(1));
    }

    // --- save/load round-trip ---

    #[test]
    fn test_save_and_reload_preserves_bills() {
        let alice = uid(1);
        let bob = uid(2);
        let mut doc = doc_with_members(&[alice, bob]);
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
        assert_eq!(bills.len(), 1);
        assert_eq!(bills[0].id, bill_id);
        assert_eq!(bills[0].amount_cents, 6000);
    }
}
