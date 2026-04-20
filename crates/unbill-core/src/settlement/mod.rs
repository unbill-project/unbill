// Settlement algorithm: who owes whom after applying all bills.
// See DESIGN.md §8 for the minimum-cash-flow greedy algorithm.

use std::collections::HashMap;

use crate::model::{Bill, EffectiveBills, Ulid, User};

/// A single suggested settlement transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub from_user_id: Ulid,
    pub to_user_id: Ulid,
    pub amount_cents: i64,
}

/// The result of computing settlement.
#[derive(Clone, Debug, Default)]
pub struct Settlement {
    pub transactions: Vec<Transaction>,
}

/// Accumulate net balances (positive = owed money, negative = owes money) from
/// one set of users + bills into an existing balance map.
///
/// Calling this for multiple ledgers and passing the same map each time produces
/// cross-ledger aggregated balances.
pub fn accumulate_balances(
    users: &[User],
    bills: &EffectiveBills,
    balances: &mut HashMap<Ulid, i64>,
) {
    for user in users.iter() {
        balances.entry(user.user_id).or_insert(0);
    }
    for bill in bills.iter() {
        let share_cents = split_amounts(bill);
        *balances.entry(bill.payer_user_id).or_default() += bill.amount_cents;
        for (user_id, amount) in share_cents {
            *balances.entry(user_id).or_default() -= amount;
        }
    }
}

/// Compute minimum-cash-flow settlement from a pre-built balance map.
pub fn compute_from_balances(balances: HashMap<Ulid, i64>) -> Settlement {
    let mut creditors: Vec<(Ulid, i64)> = balances
        .iter()
        .filter(|&(_, &b)| b > 0)
        .map(|(id, &b)| (*id, b))
        .collect();
    let mut debtors: Vec<(Ulid, i64)> = balances
        .iter()
        .filter(|&(_, &b)| b < 0)
        .map(|(id, &b)| (*id, -b))
        .collect();

    creditors.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    debtors.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut transactions = Vec::new();
    let mut ci = 0;
    let mut di = 0;

    while ci < creditors.len() && di < debtors.len() {
        let (creditor_id, ref mut credit) = creditors[ci];
        let (debtor_id, ref mut debt) = debtors[di];

        let amount = (*credit).min(*debt);
        transactions.push(Transaction {
            from_user_id: debtor_id,
            to_user_id: creditor_id,
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
pub fn split_amounts(bill: &Bill) -> Vec<(Ulid, i64)> {
    let total_shares: u32 = bill.shares.iter().map(|s| s.shares).sum();
    if total_shares == 0 {
        return bill.shares.iter().map(|s| (s.user_id, 0)).collect();
    }
    let mut amounts: Vec<(Ulid, i64)> = bill
        .shares
        .iter()
        .map(|s| {
            let amount = (bill.amount_cents * s.shares as i64) / total_shares as i64;
            (s.user_id, amount)
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
    use crate::model::{Bill, EffectiveBills, NodeId, Share, Timestamp, Ulid, User};

    /// Convenience: compute settlement from a single set of users + bills.
    fn compute(users: &[User], bills: &EffectiveBills) -> Settlement {
        let mut balances = HashMap::new();
        accumulate_balances(users, bills, &mut balances);
        compute_from_balances(balances)
    }

    fn device() -> NodeId {
        NodeId::from_seed(1)
    }

    fn uid(n: u128) -> Ulid {
        Ulid::from_u128(n)
    }

    fn user(id: Ulid) -> User {
        User {
            user_id: id,
            display_name: String::new(),
            added_at: Timestamp::from_millis(0),
            added_by: uid(0),
        }
    }

    /// Equal split: give every participant 1 share.
    fn equal_bill(id: u128, payer: Ulid, amount_cents: i64, participants: &[Ulid]) -> Bill {
        Bill {
            id: uid(id),
            payer_user_id: payer,
            amount_cents,
            description: String::new(),
            shares: participants
                .iter()
                .map(|&u| Share {
                    user_id: u,
                    shares: 1,
                })
                .collect(),
            prev: vec![],
            created_at: Timestamp::from_millis(0),
            created_by_device: device(),
        }
    }

    // Named test participants.
    fn alice() -> Ulid {
        uid(1)
    }
    fn bob() -> Ulid {
        uid(2)
    }
    fn carol() -> Ulid {
        uid(3)
    }

    // --- split_amounts ---

    #[test]
    fn test_split_equal_exact_division() {
        let bill = equal_bill(1, alice(), 300, &[alice(), bob(), carol()]);
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
        let bill = equal_bill(1, alice(), 1000, &[alice(), bob(), carol()]);
        let shares = split_amounts(&bill);
        let total: i64 = shares.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 1000);
        assert_eq!(shares[0].1, 334);
        assert_eq!(shares[1].1, 333);
        assert_eq!(shares[2].1, 333);
    }

    #[test]
    fn test_split_proportional_shares() {
        let bill = Bill {
            id: uid(1),
            payer_user_id: alice(),
            amount_cents: 300,
            description: String::new(),
            shares: vec![
                Share {
                    user_id: alice(),
                    shares: 2,
                },
                Share {
                    user_id: bob(),
                    shares: 1,
                },
            ],
            prev: vec![],
            created_at: Timestamp::from_millis(0),
            created_by_device: device(),
        };
        let amounts = split_amounts(&bill);
        let a = amounts.iter().find(|(id, _)| *id == alice()).unwrap().1;
        let b = amounts.iter().find(|(id, _)| *id == bob()).unwrap().1;
        assert_eq!(a, 200);
        assert_eq!(b, 100);
        assert_eq!(a + b, 300);
    }

    // --- compute (settlement) ---

    fn net_transfer_balances(
        users: &[User],
        bills: &EffectiveBills,
    ) -> std::collections::HashMap<Ulid, i64> {
        let s = compute(users, bills);
        let mut bal: std::collections::HashMap<Ulid, i64> =
            users.iter().map(|user| (user.user_id, 0)).collect();
        for t in &s.transactions {
            *bal.entry(t.from_user_id).or_default() -= t.amount_cents;
            *bal.entry(t.to_user_id).or_default() += t.amount_cents;
        }
        bal
    }

    #[test]
    fn test_settlement_balances_to_zero() {
        // Alice paid $90 for all three; each owes $30. Net: alice +60, bob -30, carol -30.
        let users = vec![user(alice()), user(bob()), user(carol())];
        let bills = EffectiveBills(vec![equal_bill(
            1,
            alice(),
            9000,
            &[alice(), bob(), carol()],
        )]);
        let s = compute(&users, &bills);
        let total_to_alice: i64 = s
            .transactions
            .iter()
            .filter(|t| t.to_user_id == alice())
            .map(|t| t.amount_cents)
            .sum();
        assert_eq!(total_to_alice, 6000);
        assert!(s.transactions.iter().all(|t| t.amount_cents > 0));
    }

    #[test]
    fn test_settlement_net_sum_zero() {
        let users = vec![user(alice()), user(bob()), user(carol())];
        let bills = EffectiveBills(vec![
            equal_bill(1, alice(), 6000, &[alice(), bob(), carol()]),
            equal_bill(2, bob(), 3000, &[alice(), bob()]),
        ]);
        let net = net_transfer_balances(&users, &bills);
        let sum: i64 = net.values().sum();
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_settlement_at_most_n_minus_one_transactions() {
        let uids: Vec<Ulid> = (0..5u128).map(uid).collect();
        let users: Vec<User> = uids.iter().map(|&id| user(id)).collect();
        let bill = equal_bill(1, uids[0], 5000, &uids);
        let s = compute(&users, &EffectiveBills(vec![bill]));
        assert!(
            s.transactions.len() <= users.len() - 1,
            "got {} transactions for {} users",
            s.transactions.len(),
            users.len()
        );
    }

    #[test]
    fn test_settlement_already_settled() {
        let users = vec![user(alice()), user(bob())];
        let bills = EffectiveBills(vec![
            equal_bill(1, alice(), 3000, &[alice(), bob()]),
            equal_bill(2, bob(), 3000, &[alice(), bob()]),
        ]);
        let s = compute(&users, &bills);
        assert!(s.transactions.is_empty());
    }
}
