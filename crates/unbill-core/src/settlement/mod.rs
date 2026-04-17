// Settlement algorithm: who owes whom after applying all bills.
// See DESIGN.md §8 for the minimum-cash-flow greedy algorithm.

use crate::model::{EffectiveBill, Member};

/// A single suggested settlement transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub from_user_id: String,
    pub to_user_id: String,
    pub amount_cents: i64,
}

/// The result of computing settlement for a ledger.
#[derive(Clone, Debug, Default)]
pub struct Settlement {
    pub transactions: Vec<Transaction>,
}

/// Compute minimum-cash-flow settlement from a list of effective bills and members.
pub fn compute(members: &[Member], bills: &[EffectiveBill]) -> Settlement {
    // Step 1: net balance per user (positive = owed money, negative = owes money).
    let mut balances: std::collections::HashMap<String, i64> = members
        .iter()
        .filter(|m| !m.removed)
        .map(|m| (m.user_id.clone(), 0i64))
        .collect();

    for bill in bills {
        if bill.is_deleted {
            continue;
        }
        let share_cents = split_amounts(bill);
        *balances.entry(bill.payer_user_id.clone()).or_default() += bill.amount_cents;
        for (user_id, amount) in share_cents {
            *balances.entry(user_id).or_default() -= amount;
        }
    }

    // Step 2: greedy minimum cash flow.
    let mut creditors: Vec<(String, i64)> = balances
        .iter()
        .filter(|(_, &b)| b > 0)
        .map(|(id, &b)| (id.clone(), b))
        .collect();
    let mut debtors: Vec<(String, i64)> = balances
        .iter()
        .filter(|(_, &b)| b < 0)
        .map(|(id, &b)| (id.clone(), -b))
        .collect();

    creditors.sort_by(|a, b| b.1.cmp(&a.1));
    debtors.sort_by(|a, b| b.1.cmp(&a.1));

    let mut transactions = Vec::new();
    let mut ci = 0;
    let mut di = 0;

    while ci < creditors.len() && di < debtors.len() {
        let (ref creditor_id, ref mut credit) = creditors[ci];
        let (ref debtor_id, ref mut debt) = debtors[di];

        let amount = (*credit).min(*debt);
        transactions.push(Transaction {
            from_user_id: debtor_id.clone(),
            to_user_id: creditor_id.clone(),
            amount_cents: amount,
        });

        *credit -= amount;
        *debt -= amount;

        if *credit == 0 {
            ci += 1;
        }
        if *debt == 0 {
            di += 1;
        }
    }

    Settlement { transactions }
}

/// Compute the per-participant cent amounts for a bill from its share weights.
/// Rounding remainder (from integer division) is assigned to the earliest participants.
pub fn split_amounts(bill: &EffectiveBill) -> Vec<(String, i64)> {
    let total_shares: u32 = bill.shares.iter().map(|s| s.shares).sum();
    if total_shares == 0 {
        return bill.shares.iter().map(|s| (s.user_id.clone(), 0)).collect();
    }
    let mut amounts: Vec<(String, i64)> = bill
        .shares
        .iter()
        .map(|s| {
            let amount = (bill.amount_cents * s.shares as i64) / total_shares as i64;
            (s.user_id.clone(), amount)
        })
        .collect();
    // Distribute rounding remainder to the earliest participants.
    let assigned: i64 = amounts.iter().map(|(_, a)| a).sum();
    let mut remainder = bill.amount_cents - assigned;
    for (_, amount) in amounts.iter_mut() {
        if remainder == 0 {
            break;
        }
        *amount += 1;
        remainder -= 1;
    }
    amounts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EffectiveBill, Member, Share, Timestamp};

    fn member(id: &str) -> Member {
        Member {
            user_id: id.to_string(),
            display_name: id.to_string(),
            devices: vec![],
            added_at: Timestamp::from_millis(0),
            added_by: String::new(),
            removed: false,
        }
    }

    /// Equal split: give every participant 1 share.
    fn equal_bill(
        id: &str,
        payer: &str,
        amount_cents: i64,
        participants: &[&str],
    ) -> EffectiveBill {
        EffectiveBill {
            id: id.to_string(),
            payer_user_id: payer.to_string(),
            amount_cents,
            description: String::new(),
            shares: participants
                .iter()
                .map(|u| Share {
                    user_id: u.to_string(),
                    shares: 1,
                })
                .collect(),
            was_amended: false,
            is_deleted: false,
            last_modified_at: Timestamp::from_millis(0),
            history: vec![],
        }
    }

    // --- split_amounts ---

    #[test]
    fn test_split_equal_exact_division() {
        let bill = equal_bill("b1", "alice", 300, &["alice", "bob", "carol"]);
        let shares = split_amounts(&bill);
        assert_eq!(shares.len(), 3);
        for (_, cents) in &shares {
            assert_eq!(*cents, 100);
        }
        let total: i64 = shares.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 300);
    }

    #[test]
    fn test_split_remainder_distributed_to_earliest() {
        // $10 split 3 ways: 334, 333, 333
        let bill = equal_bill("b1", "alice", 1000, &["alice", "bob", "carol"]);
        let shares = split_amounts(&bill);
        let total: i64 = shares.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 1000);
        assert_eq!(shares[0].1, 334);
        assert_eq!(shares[1].1, 333);
        assert_eq!(shares[2].1, 333);
    }

    #[test]
    fn test_split_proportional_shares() {
        let bill = EffectiveBill {
            id: "b1".into(),
            payer_user_id: "alice".into(),
            amount_cents: 300,
            description: String::new(),
            shares: vec![
                Share {
                    user_id: "alice".into(),
                    shares: 2,
                },
                Share {
                    user_id: "bob".into(),
                    shares: 1,
                },
            ],
            was_amended: false,
            is_deleted: false,
            last_modified_at: Timestamp::from_millis(0),
            history: vec![],
        };
        let amounts = split_amounts(&bill);
        let alice = amounts.iter().find(|(id, _)| id == "alice").unwrap().1;
        let bob = amounts.iter().find(|(id, _)| id == "bob").unwrap().1;
        assert_eq!(alice, 200);
        assert_eq!(bob, 100);
        assert_eq!(alice + bob, 300);
    }

    // --- compute (settlement) ---

    fn net_transfer_balances(
        members: &[Member],
        bills: &[EffectiveBill],
    ) -> std::collections::HashMap<String, i64> {
        let s = compute(members, bills);
        let mut bal: std::collections::HashMap<String, i64> =
            members.iter().map(|m| (m.user_id.clone(), 0)).collect();
        for t in &s.transactions {
            *bal.entry(t.from_user_id.clone()).or_default() -= t.amount_cents;
            *bal.entry(t.to_user_id.clone()).or_default() += t.amount_cents;
        }
        bal
    }

    #[test]
    fn test_settlement_balances_to_zero() {
        // Alice paid $90 for all three; each owes $30. Net: alice +60, bob -30, carol -30.
        let members = vec![member("alice"), member("bob"), member("carol")];
        let bills = vec![equal_bill("b1", "alice", 9000, &["alice", "bob", "carol"])];
        let s = compute(&members, &bills);
        let total_to_alice: i64 = s
            .transactions
            .iter()
            .filter(|t| t.to_user_id == "alice")
            .map(|t| t.amount_cents)
            .sum();
        assert_eq!(total_to_alice, 6000);
        assert!(s.transactions.iter().all(|t| t.amount_cents > 0));
    }

    #[test]
    fn test_settlement_net_sum_zero() {
        let members = vec![member("alice"), member("bob"), member("carol")];
        let bills = vec![
            equal_bill("b1", "alice", 6000, &["alice", "bob", "carol"]),
            equal_bill("b2", "bob", 3000, &["alice", "bob"]),
        ];
        let net = net_transfer_balances(&members, &bills);
        let sum: i64 = net.values().sum();
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_settlement_at_most_n_minus_one_transactions() {
        let members: Vec<Member> = (0..5).map(|i| member(&format!("u{i}"))).collect();
        let bill = equal_bill("b1", "u0", 5000, &["u0", "u1", "u2", "u3", "u4"]);
        let s = compute(&members, &[bill]);
        assert!(
            s.transactions.len() <= members.len() - 1,
            "got {} transactions for {} members",
            s.transactions.len(),
            members.len()
        );
    }

    #[test]
    fn test_settlement_already_settled() {
        let members = vec![member("alice"), member("bob")];
        let bills = vec![
            equal_bill("b1", "alice", 3000, &["alice", "bob"]),
            equal_bill("b2", "bob", 3000, &["alice", "bob"]),
        ];
        let s = compute(&members, &bills);
        assert!(s.transactions.is_empty());
    }

    #[test]
    fn test_settlement_deleted_bills_ignored() {
        let members = vec![member("alice"), member("bob")];
        let mut bill = equal_bill("b1", "alice", 10000, &["alice", "bob"]);
        bill.is_deleted = true;
        let s = compute(&members, &[bill]);
        assert!(
            s.transactions.is_empty(),
            "deleted bills must not affect settlement"
        );
    }

    #[test]
    fn test_settlement_removed_members_excluded() {
        let mut eve = member("eve");
        eve.removed = true;
        let members = vec![member("alice"), member("bob"), eve];
        let bills = vec![equal_bill("b1", "alice", 3000, &["alice", "bob"])];
        let s = compute(&members, &bills);
        assert!(
            s.transactions
                .iter()
                .all(|t| t.from_user_id != "eve" && t.to_user_id != "eve"),
            "removed members must not appear in settlement"
        );
    }
}
