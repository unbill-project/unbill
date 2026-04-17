use autosurgeon::{Hydrate, Reconcile};

use super::bill::{Bill, Share};
use super::id::Ulid;
use super::timestamp::Timestamp;

#[derive(Clone, Debug, Reconcile, Hydrate)]
pub struct Amendment {
    pub id: Ulid,
    pub new_amount_cents: Option<i64>,
    pub new_description: Option<String>,
    /// Replaces the entire shares list. Changing participants means changing shares.
    pub new_shares: Option<Vec<Share>>,
    pub author_user_id: Ulid,
    pub created_at: Timestamp,
    pub reason: Option<String>,
}

/// User-facing input for creating an Amendment via the service layer.
#[derive(Clone, Debug)]
pub struct BillAmendment {
    pub new_amount_cents: Option<i64>,
    pub new_description: Option<String>,
    pub new_shares: Option<Vec<Share>>,
    pub author_user_id: Ulid,
    pub reason: Option<String>,
}

/// The rendered view of a Bill after all amendments have been applied.
/// This is what frontends display — never raw `Bill` structs.
///
/// # Projection rules (see DESIGN.md §4.4)
/// - Amendments are sorted by `created_at` ascending; ties broken by `id` lexically.
/// - Each amendment field, if `Some`, overwrites the current value.
#[derive(Clone, Debug)]
pub struct EffectiveBill {
    pub id: Ulid,
    pub payer_user_id: Ulid,
    pub amount_cents: i64,
    pub description: String,
    pub shares: Vec<Share>,
    pub was_amended: bool,
    pub is_deleted: bool,
    pub last_modified_at: Timestamp,
    pub history: Vec<AmendmentSummary>,
}

impl EffectiveBill {
    /// Convenience: participant user IDs derived from shares.
    pub fn participants(&self) -> Vec<Ulid> {
        self.shares.iter().map(|s| s.user_id).collect()
    }

    /// Project a `Bill` (with its amendment log) into the effective view.
    pub fn from(bill: &Bill) -> Self {
        let mut amount_cents = bill.amount_cents;
        let mut description = bill.description.clone();
        let mut shares = bill.shares.clone();
        let is_deleted = bill.deleted;
        let mut last_modified_at = bill.created_at;
        let mut was_amended = false;

        // Sort amendments: primary key = created_at asc, secondary = id asc.
        // Both Timestamp and Ulid implement Ord, so direct comparison works.
        let mut sorted_amendments = bill.amendments.clone();
        sorted_amendments.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut history = Vec::with_capacity(sorted_amendments.len());

        for amend in &sorted_amendments {
            was_amended = true;
            if let Some(v) = amend.new_amount_cents {
                amount_cents = v;
            }
            if let Some(ref v) = amend.new_description {
                description = v.clone();
            }
            if let Some(ref v) = amend.new_shares {
                shares = v.clone();
            }
            if amend.created_at > last_modified_at {
                last_modified_at = amend.created_at;
            }
            history.push(AmendmentSummary {
                id: amend.id,
                author_user_id: amend.author_user_id,
                created_at: amend.created_at,
                reason: amend.reason.clone(),
            });
        }

        EffectiveBill {
            id: bill.id,
            payer_user_id: bill.payer_user_id,
            amount_cents,
            description,
            shares,
            was_amended,
            is_deleted,
            last_modified_at,
            history,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AmendmentSummary {
    pub id: Ulid,
    pub author_user_id: Ulid,
    pub created_at: Timestamp,
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::bill::{Bill, Share};
    use crate::model::id::Ulid;
    use crate::model::timestamp::Timestamp;

    // Deterministic test IDs. Use small integers to keep tests readable.
    fn uid(n: u128) -> Ulid {
        Ulid::from_u128(n)
    }

    fn ts(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    fn share(user_id: Ulid) -> Share {
        Share { user_id, shares: 1 }
    }

    fn base_bill() -> Bill {
        Bill {
            id: uid(100),
            payer_user_id: uid(1), // alice
            amount_cents: 3000,
            description: "Dinner".into(),
            shares: vec![share(uid(1)), share(uid(2))], // alice, bob
            created_at: ts(1000),
            created_by_device: "device-a".into(),
            deleted: false,
            amendments: vec![],
        }
    }

    fn amend(id: u128, millis: i64) -> Amendment {
        Amendment {
            id: uid(id),
            new_amount_cents: None,
            new_description: None,
            new_shares: None,
            author_user_id: uid(1), // alice
            created_at: ts(millis),
            reason: None,
        }
    }

    #[test]
    fn test_effective_bill_no_amendments() {
        let bill = base_bill();
        let eff = EffectiveBill::from(&bill);
        assert_eq!(eff.amount_cents, 3000);
        assert_eq!(eff.description, "Dinner");
        assert!(!eff.was_amended);
        assert!(!eff.is_deleted);
        assert!(eff.history.is_empty());
        assert_eq!(eff.last_modified_at, ts(1000));
    }

    #[test]
    fn test_effective_bill_single_amendment_updates_fields() {
        let mut bill = base_bill();
        bill.amendments.push(Amendment {
            new_amount_cents: Some(4500),
            new_description: Some("Dinner + drinks".into()),
            ..amend(10, 2000)
        });
        let eff = EffectiveBill::from(&bill);
        assert_eq!(eff.amount_cents, 4500);
        assert_eq!(eff.description, "Dinner + drinks");
        assert!(eff.was_amended);
        assert_eq!(eff.last_modified_at, ts(2000));
        assert_eq!(eff.history.len(), 1);
    }

    #[test]
    fn test_effective_bill_amendments_applied_in_timestamp_order() {
        let mut bill = base_bill();
        bill.amendments.push(Amendment {
            new_amount_cents: Some(9999),
            ..amend(20, 3000)
        });
        bill.amendments.push(Amendment {
            new_amount_cents: Some(4500),
            ..amend(10, 2000)
        });
        let eff = EffectiveBill::from(&bill);
        assert_eq!(eff.amount_cents, 9999, "later amendment should win");
    }

    #[test]
    fn test_effective_bill_tie_broken_by_id_order() {
        let mut bill = base_bill();
        // uid(999) > uid(1), so uid(1) is applied first and uid(999) overwrites → result 100
        bill.amendments.push(Amendment {
            new_amount_cents: Some(100),
            ..amend(999, 2000)
        });
        bill.amendments.push(Amendment {
            new_amount_cents: Some(200),
            ..amend(1, 2000)
        });
        let eff = EffectiveBill::from(&bill);
        assert_eq!(
            eff.amount_cents, 100,
            "higher-id amendment should win on timestamp tie"
        );
    }

    #[test]
    fn test_effective_bill_partial_amendment_leaves_other_fields_unchanged() {
        let mut bill = base_bill();
        bill.amendments.push(Amendment {
            new_description: Some("Updated description".into()),
            ..amend(10, 2000)
        });
        let eff = EffectiveBill::from(&bill);
        assert_eq!(eff.amount_cents, 3000);
        assert_eq!(eff.description, "Updated description");
        assert_eq!(eff.participants(), vec![uid(1), uid(2)]);
    }

    #[test]
    fn test_effective_bill_preserves_deleted_tombstone() {
        let mut bill = base_bill();
        bill.deleted = true;
        let eff = EffectiveBill::from(&bill);
        assert!(eff.is_deleted);
    }

    #[test]
    fn test_effective_bill_history_in_applied_order() {
        let mut bill = base_bill();
        bill.amendments.push(amend(20, 3000));
        bill.amendments.push(amend(10, 2000));
        let eff = EffectiveBill::from(&bill);
        assert_eq!(eff.history[0].id, uid(10));
        assert_eq!(eff.history[1].id, uid(20));
    }

    #[test]
    fn test_effective_bill_last_modified_at_is_latest_amendment_ts() {
        let mut bill = base_bill();
        bill.amendments.push(amend(10, 5000));
        bill.amendments.push(amend(20, 3000));
        let eff = EffectiveBill::from(&bill);
        assert_eq!(eff.last_modified_at, ts(5000));
    }

    #[test]
    fn test_effective_bill_shares_amendment_updates_participants() {
        let carol = uid(3);
        let mut bill = base_bill(); // uid(1) + uid(2), 1 share each
        bill.amendments.push(Amendment {
            new_shares: Some(vec![
                Share {
                    user_id: uid(1),
                    shares: 2,
                },
                Share {
                    user_id: uid(2),
                    shares: 1,
                },
                Share {
                    user_id: carol,
                    shares: 1,
                },
            ]),
            ..amend(10, 2000)
        });
        let eff = EffectiveBill::from(&bill);
        assert_eq!(eff.shares.len(), 3);
        assert_eq!(eff.participants(), vec![uid(1), uid(2), carol]);
    }
}
