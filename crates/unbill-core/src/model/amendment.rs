use super::bill::{Bill, Share};
use super::id::Ulid;
use super::timestamp::Timestamp;

/// The rendered view of a bill after all revisions have been applied.
/// Multiple `Bill` entries in the CRDT may share the same `id`; the one with
/// the latest `created_at` (ties broken by `created_by_device` string) wins.
#[derive(Clone, Debug)]
pub struct EffectiveBill {
    pub id: Ulid,
    pub payer_user_id: Ulid,
    pub amount_cents: i64,
    pub description: String,
    pub shares: Vec<Share>,
    /// True when more than one entry with this bill ID exists.
    pub was_amended: bool,
    pub last_modified_at: Timestamp,
}

impl EffectiveBill {
    /// Convenience: participant user IDs derived from shares.
    pub fn participants(&self) -> Vec<Ulid> {
        self.shares.iter().map(|s| s.user_id).collect()
    }

    /// Project a group of `Bill` entries (all sharing the same logical `id`)
    /// into a single effective view.  The entry with the latest `created_at`
    /// wins; ties are broken lexicographically by `created_by_device`.
    ///
    /// # Panics
    /// Panics if `entries` is empty.
    pub fn project(mut entries: Vec<Bill>) -> Self {
        entries.sort_by(|a, b| {
            a.created_at.cmp(&b.created_at).then_with(|| {
                a.created_by_device
                    .to_string()
                    .cmp(&b.created_by_device.to_string())
            })
        });
        let was_amended = entries.len() > 1;
        let first = entries.first().unwrap();
        let last = entries.last().unwrap();
        EffectiveBill {
            id: first.id,
            payer_user_id: last.payer_user_id,
            amount_cents: last.amount_cents,
            description: last.description.clone(),
            shares: last.shares.clone(),
            was_amended,
            last_modified_at: last.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::bill::{Bill, Share};
    use crate::model::id::Ulid;
    use crate::model::node_id::NodeId;
    use crate::model::timestamp::Timestamp;

    fn uid(n: u128) -> Ulid {
        Ulid::from_u128(n)
    }

    fn ts(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    fn share(user_id: Ulid) -> Share {
        Share { user_id, shares: 1 }
    }

    fn dev(seed: u8) -> NodeId {
        NodeId::from_seed(seed)
    }

    fn bill(id: u128, payer: u128, amount: i64, desc: &str, millis: i64, device: u8) -> Bill {
        Bill {
            id: uid(id),
            payer_user_id: uid(payer),
            amount_cents: amount,
            description: desc.to_string(),
            shares: vec![share(uid(payer))],
            created_at: ts(millis),
            created_by_device: dev(device),
        }
    }

    #[test]
    fn test_single_entry_is_not_amended() {
        let eff = EffectiveBill::project(vec![bill(1, 10, 3000, "Dinner", 1000, 1)]);
        assert_eq!(eff.amount_cents, 3000);
        assert_eq!(eff.description, "Dinner");
        assert!(!eff.was_amended);
        assert_eq!(eff.last_modified_at, ts(1000));
    }

    #[test]
    fn test_later_entry_wins() {
        let original = bill(1, 10, 3000, "Dinner", 1000, 1);
        let amendment = bill(1, 10, 4500, "Dinner + drinks", 2000, 1);
        let eff = EffectiveBill::project(vec![original, amendment]);
        assert_eq!(eff.amount_cents, 4500);
        assert_eq!(eff.description, "Dinner + drinks");
        assert!(eff.was_amended);
        assert_eq!(eff.last_modified_at, ts(2000));
    }

    #[test]
    fn test_entries_sorted_by_timestamp_regardless_of_input_order() {
        let a = bill(1, 10, 9999, "Wrong", 3000, 1);
        let b = bill(1, 10, 4500, "Right", 2000, 1);
        // 'a' has a later timestamp so it wins even though it's listed last
        let eff = EffectiveBill::project(vec![b, a]);
        assert_eq!(eff.amount_cents, 9999);
    }

    #[test]
    fn test_tie_broken_deterministically_regardless_of_input_order() {
        // Two entries at the same timestamp — result must be order-independent.
        let a = bill(1, 10, 100, "Entry A", 2000, 1);
        let b = bill(1, 10, 200, "Entry B", 2000, 2);
        let eff1 = EffectiveBill::project(vec![a.clone(), b.clone()]);
        let eff2 = EffectiveBill::project(vec![b, a]);
        assert_eq!(
            eff1.amount_cents, eff2.amount_cents,
            "tiebreaker must produce the same winner regardless of input order"
        );
    }
}
