// Settlement algorithm: who owes whom after applying all bills.
// See DESIGN.md for the two-step algorithm.

use std::collections::{HashMap, HashSet};

use crate::model::{BillId, Currency, EffectiveBills, Ledger, User, UserId};

/// A single suggested settlement transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub amount_cents: i64,
}

/// The result of computing settlement.
#[derive(Clone, Debug)]
pub struct Settlement {
    pub currency: Currency,
    pub transactions: Vec<Transaction>,
}

/// The per-user cent amounts for both sides of a single bill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BillSplit {
    /// How much each payer is credited.
    pub payer_amounts: Vec<(UserId, i64)>,
    /// How much each payee owes.
    pub payee_amounts: Vec<(UserId, i64)>,
}

/// Compute the exact cent amounts for each payer and payee of a bill.
///
/// Uses the bill's own ID as the rounding seed, so the result is identical
/// to what `compute_settlement` would derive for the same bill.
pub fn compute_bill_split(
    payers: &[crate::model::Share],
    payees: &[crate::model::Share],
    total_cents: i64,
    bill_id: BillId,
) -> BillSplit {
    BillSplit {
        payer_amounts: split_shares(payers, total_cents, bill_id),
        payee_amounts: split_shares(payees, total_cents, bill_id),
    }
}

/// Compute settlement for a single ledger.
///
/// Derives effective bills from `ledger.bills`, accumulates per-user balances,
/// and applies the greedy minimum-cash-flow reduction.
pub fn compute_settlement(ledger: &Ledger) -> Settlement {
    let superseded: HashSet<BillId> = ledger
        .bills
        .iter()
        .flat_map(|b| b.prev.iter().copied())
        .collect();
    let mut balances: HashMap<UserId, i64> = HashMap::new();
    for bill in ledger.bills.iter().filter(|b| !superseded.contains(&b.id)) {
        for (user_id, amount) in split_shares(&bill.payers, bill.amount_cents, bill.id) {
            *balances.entry(user_id).or_default() += amount;
        }
        for (user_id, amount) in split_shares(&bill.payees, bill.amount_cents, bill.id) {
            *balances.entry(user_id).or_default() -= amount;
        }
    }
    compute_from_balances(ledger.currency, balances)
}

/// Accumulate net balances (positive = owed money, negative = owes money) from
/// one set of users + bills into an existing balance map.
///
/// Calling this for multiple ledgers and passing the same map each time produces
/// cross-ledger aggregated balances.
pub fn accumulate_balances(
    users: &[User],
    bills: &EffectiveBills,
    balances: &mut HashMap<UserId, i64>,
) {
    for user in users.iter() {
        balances.entry(user.user_id).or_insert(0);
    }
    for bill in bills.iter() {
        for (user_id, amount) in split_shares(&bill.payers, bill.amount_cents, bill.id) {
            *balances.entry(user_id).or_default() += amount;
        }
        for (user_id, amount) in split_shares(&bill.payees, bill.amount_cents, bill.id) {
            *balances.entry(user_id).or_default() -= amount;
        }
    }
}

/// Compute minimum-cash-flow settlement from a pre-built balance map.
pub fn compute_from_balances(currency: Currency, balances: HashMap<UserId, i64>) -> Settlement {
    let mut creditors: Vec<(UserId, i64)> = balances
        .iter()
        .filter(|&(_, &b)| b > 0)
        .map(|(id, &b)| (*id, b))
        .collect();
    let mut debtors: Vec<(UserId, i64)> = balances
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

    Settlement {
        currency,
        transactions,
    }
}

/// Compute the per-user cent amounts from a share list and a total.
///
/// Each user receives `floor((total_cents × share_weight) / total_weight)`.
/// The rounding remainder is assigned in full to a single user selected by
/// `fnv1a(bill_id) mod len(shares)`, making the result deterministic across
/// all peers for the same bill.
pub fn split_shares(
    shares: &[crate::model::Share],
    total_cents: i64,
    bill_id: BillId,
) -> Vec<(UserId, i64)> {
    if shares.is_empty() {
        return vec![];
    }
    let total_weight: u32 = shares.iter().map(|s| s.shares).sum();
    if total_weight == 0 {
        return shares.iter().map(|s| (s.user_id, 0)).collect();
    }
    let mut amounts: Vec<(UserId, i64)> = shares
        .iter()
        .map(|s| {
            let amount = (total_cents * s.shares as i64) / total_weight as i64;
            (s.user_id, amount)
        })
        .collect();
    let assigned: i64 = amounts.iter().map(|(_, a)| a).sum();
    let remainder = total_cents - assigned;
    if remainder != 0 {
        let idx = fnv1a(bill_id.to_string().as_bytes()) as usize % shares.len();
        amounts[idx].1 += remainder;
    }
    amounts
}

/// FNV-1a hash over a byte slice. Used to deterministically select the
/// remainder recipient in `split_shares`.
fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bill, BillId, LedgerId, NodeId, Share, Timestamp, User, UserId};

    fn device() -> NodeId {
        NodeId::from_seed(1)
    }

    fn uid(n: u128) -> UserId {
        UserId::from_u128(n)
    }

    fn bid(n: u128) -> BillId {
        BillId::from_u128(n)
    }

    fn lid(n: u128) -> LedgerId {
        LedgerId::from_u128(n)
    }

    fn alice() -> UserId {
        uid(1)
    }
    fn bob() -> UserId {
        uid(2)
    }
    fn carol() -> UserId {
        uid(3)
    }

    fn user(id: UserId) -> User {
        User {
            user_id: id,
            display_name: String::new(),
            added_at: Timestamp::from_millis(0),
        }
    }

    fn make_ledger(users: Vec<User>, bills: Vec<Bill>) -> Ledger {
        Ledger {
            ledger_id: lid(999),
            schema_version: 1,
            name: String::new(),
            currency: Currency::from_code("USD").unwrap(),
            created_at: Timestamp::from_millis(0),
            users,
            bills,
            devices: vec![],
        }
    }

    fn equal_bill(id: u128, payer: UserId, amount_cents: i64, payee_users: &[UserId]) -> Bill {
        Bill {
            id: bid(id),
            amount_cents,
            description: String::new(),
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
            created_at: Timestamp::from_millis(0),
            created_by_device: device(),
        }
    }

    // --- split_shares ---

    #[test]
    fn test_split_exact_division_no_remainder() {
        // $3.00 split 3 ways: each gets exactly $1.00, no remainder.
        let shares = vec![
            Share {
                user_id: alice(),
                shares: 1,
            },
            Share {
                user_id: bob(),
                shares: 1,
            },
            Share {
                user_id: carol(),
                shares: 1,
            },
        ];
        let amounts = split_shares(&shares, 300, bid(1));
        let total: i64 = amounts.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 300);
        for (_, cents) in &amounts {
            assert_eq!(*cents, 100);
        }
    }

    #[test]
    fn test_split_remainder_assigned_to_single_user() {
        // $10.00 split 3 ways: floor gives 333 each (sum 999), remainder 1 goes to exactly one user.
        let shares = vec![
            Share {
                user_id: alice(),
                shares: 1,
            },
            Share {
                user_id: bob(),
                shares: 1,
            },
            Share {
                user_id: carol(),
                shares: 1,
            },
        ];
        let amounts = split_shares(&shares, 1000, bid(1));
        let total: i64 = amounts.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 1000);
        let high_count = amounts.iter().filter(|(_, c)| *c == 334).count();
        let low_count = amounts.iter().filter(|(_, c)| *c == 333).count();
        assert_eq!(
            high_count, 1,
            "exactly one user should receive the remainder"
        );
        assert_eq!(low_count, 2);
    }

    #[test]
    fn test_split_is_deterministic() {
        // Same bill ID always produces the same recipient for the remainder.
        let shares = vec![
            Share {
                user_id: alice(),
                shares: 1,
            },
            Share {
                user_id: bob(),
                shares: 1,
            },
            Share {
                user_id: carol(),
                shares: 1,
            },
        ];
        let first = split_shares(&shares, 1000, bid(42));
        let second = split_shares(&shares, 1000, bid(42));
        assert_eq!(first, second);
    }

    #[test]
    fn test_split_different_bill_ids_may_pick_different_recipients() {
        // Different bill IDs should not always assign the remainder to the same user.
        let shares = vec![
            Share {
                user_id: alice(),
                shares: 1,
            },
            Share {
                user_id: bob(),
                shares: 1,
            },
            Share {
                user_id: carol(),
                shares: 1,
            },
        ];
        let recipients: Vec<UserId> = (0u128..20)
            .map(|n| {
                let amounts = split_shares(&shares, 1000, bid(n));
                amounts.into_iter().find(|(_, c)| *c == 334).unwrap().0
            })
            .collect();
        let unique: std::collections::HashSet<_> = recipients.iter().collect();
        assert!(
            unique.len() > 1,
            "remainder should not always go to the same user"
        );
    }

    #[test]
    fn test_split_proportional_weights() {
        let shares = vec![
            Share {
                user_id: alice(),
                shares: 2,
            },
            Share {
                user_id: bob(),
                shares: 1,
            },
        ];
        let amounts = split_shares(&shares, 300, bid(1));
        let a = amounts.iter().find(|(id, _)| *id == alice()).unwrap().1;
        let b = amounts.iter().find(|(id, _)| *id == bob()).unwrap().1;
        assert_eq!(a, 200);
        assert_eq!(b, 100);
        assert_eq!(a + b, 300);
    }

    #[test]
    fn test_split_zero_shares_list() {
        let amounts = split_shares(&[], 1000, bid(1));
        assert!(amounts.is_empty());
    }

    // --- compute_settlement ---

    fn net_transfer_balances(ledger: &Ledger) -> HashMap<UserId, i64> {
        let s = compute_settlement(ledger);
        let mut bal: HashMap<UserId, i64> = ledger.users.iter().map(|u| (u.user_id, 0)).collect();
        for t in &s.transactions {
            *bal.entry(t.from_user_id).or_default() -= t.amount_cents;
            *bal.entry(t.to_user_id).or_default() += t.amount_cents;
        }
        bal
    }

    #[test]
    fn test_settlement_balances_to_zero() {
        // Alice paid $90 for all three; each owes $30. Net: alice +60, bob -30, carol -30.
        let ledger = make_ledger(
            vec![user(alice()), user(bob()), user(carol())],
            vec![equal_bill(1, alice(), 9000, &[alice(), bob(), carol()])],
        );
        let s = compute_settlement(&ledger);
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
        let ledger = make_ledger(
            vec![user(alice()), user(bob()), user(carol())],
            vec![
                equal_bill(1, alice(), 6000, &[alice(), bob(), carol()]),
                equal_bill(2, bob(), 3000, &[alice(), bob()]),
            ],
        );
        let net = net_transfer_balances(&ledger);
        let sum: i64 = net.values().sum();
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_settlement_at_most_n_minus_one_transactions() {
        let uids: Vec<UserId> = (0..5u128).map(uid).collect();
        let users: Vec<User> = uids.iter().map(|&id| user(id)).collect();
        let bill = equal_bill(1, uids[0], 5000, &uids);
        let ledger = make_ledger(users.clone(), vec![bill]);
        let s = compute_settlement(&ledger);
        assert!(
            s.transactions.len() < users.len(),
            "got {} transactions for {} users",
            s.transactions.len(),
            users.len()
        );
    }

    #[test]
    fn test_settlement_already_settled() {
        let ledger = make_ledger(
            vec![user(alice()), user(bob())],
            vec![
                equal_bill(1, alice(), 3000, &[alice(), bob()]),
                equal_bill(2, bob(), 3000, &[alice(), bob()]),
            ],
        );
        let s = compute_settlement(&ledger);
        assert!(s.transactions.is_empty());
    }

    #[test]
    fn test_settlement_is_deterministic() {
        let ledger = make_ledger(
            vec![user(alice()), user(bob()), user(carol())],
            vec![
                equal_bill(1, alice(), 6000, &[alice(), bob(), carol()]),
                equal_bill(2, bob(), 4000, &[alice(), bob(), carol()]),
            ],
        );
        let first = compute_settlement(&ledger);
        let second = compute_settlement(&ledger);
        assert_eq!(first.transactions, second.transactions);
    }

    #[test]
    fn test_settlement_ignores_superseded_bills() {
        // Bill 2 supersedes bill 1. Only bill 2 should be counted.
        let bill1 = equal_bill(1, alice(), 9000, &[alice(), bob(), carol()]);
        let mut bill2 = equal_bill(2, alice(), 6000, &[alice(), bob()]);
        bill2.prev = vec![bid(1)];
        let ledger = make_ledger(
            vec![user(alice()), user(bob()), user(carol())],
            vec![bill1, bill2],
        );
        let s = compute_settlement(&ledger);
        // Only bill2 counts: alice paid $60 for alice+bob, each owes $30. Net: alice +30, bob -30.
        let total_to_alice: i64 = s
            .transactions
            .iter()
            .filter(|t| t.to_user_id == alice())
            .map(|t| t.amount_cents)
            .sum();
        assert_eq!(total_to_alice, 3000);
    }
}
