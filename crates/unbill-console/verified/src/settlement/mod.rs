// Verified settlement math: split shares and minimum-cash-flow reduction.
// This is the production code — unbill-console calls these functions at runtime.

pub mod spec;

// sirno:witness:formal-invariants:begin
// sirno:witness:invariant-split-completeness:begin
// sirno:witness:invariant-conservation:begin

/// Compute the per-user cent amounts from a share list and a total.
///
/// Each user receives `floor((total_cents × share_weight) / total_weight)`.
/// The rounding remainder is assigned in full to a single user selected by
/// `remainder_recipient_idx % len(shares)`, making the result deterministic
/// when the caller derives the index from a stable seed (e.g. FNV-1a of bill ID).
///
/// # Arguments
/// * `shares` — (id, weight) pairs for each participant
/// * `total_cents` — the bill total in minor currency units
/// * `remainder_recipient_idx` — index into `shares` for the rounding remainder
pub fn split_shares<Id: Copy>(
    shares: &[(Id, u32)],
    total_cents: i64,
    remainder_recipient_idx: usize,
) -> Vec<(Id, i64)> {
    if shares.is_empty() {
        return vec![];
    }
    let total_weight: u32 = shares.iter().map(|(_, w)| w).sum();
    if total_weight == 0 {
        return shares.iter().map(|(id, _)| (*id, 0)).collect();
    }
    let mut amounts: Vec<(Id, i64)> = shares
        .iter()
        .map(|(id, weight)| {
            let amount = (total_cents * *weight as i64) / total_weight as i64;
            (*id, amount)
        })
        .collect();
    let assigned: i64 = amounts.iter().map(|(_, a)| a).sum();
    let remainder = total_cents - assigned;
    if remainder != 0 {
        let idx = remainder_recipient_idx % shares.len();
        amounts[idx].1 += remainder;
    }
    amounts
}

/// A single suggested settlement transaction.
#[derive(Clone, Debug)]
pub struct Transaction<Id> {
    pub from_id: Id,
    pub to_id: Id,
    pub amount_cents: i64,
}

/// Compute minimum-cash-flow settlement from a pre-built balance map.
///
/// Takes a list of (id, balance) pairs where positive = owed money,
/// negative = owes money. Returns the minimal set of transactions
/// that drives all balances to zero.
pub fn compute_from_balances<Id: Copy + Ord>(balances: &[(Id, i64)]) -> Vec<Transaction<Id>> {
    let mut creditors: Vec<(Id, i64)> = balances.iter().filter(|(_, b)| *b > 0).copied().collect();
    let mut debtors: Vec<(Id, i64)> = balances
        .iter()
        .filter(|(_, b)| *b < 0)
        .map(|(id, b)| (*id, -b))
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
            from_id: debtor_id,
            to_id: creditor_id,
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

    transactions
}
// sirno:witness:invariant-conservation:end
// sirno:witness:invariant-split-completeness:end
// sirno:witness:formal-invariants:end
