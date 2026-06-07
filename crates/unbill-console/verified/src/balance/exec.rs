// Verified balance settlement: greedy creditor-debtor matching.
// Takes a balance vector and produces transactions that conserve the total.

use super::proof;
use super::spec;
use vstd::prelude::*;
use vstd::slice::SliceAdditionalExecFns;

verus! {

/// A settlement transaction (runtime).
pub struct Transaction {
    pub from_user_id: Vec<u8>,
    pub to_user_id: Vec<u8>,
    pub amount_cents: i64,
}

impl View for Transaction {
    type V = spec::TransactionSpec;
    open spec fn view(&self) -> spec::TransactionSpec {
        spec::TransactionSpec {
            from_user_id: self.from_user_id@,
            to_user_id: self.to_user_id@,
            amount_cents: self.amount_cents as int,
        }
    }
}

/// Settle a balance vector into transactions.
/// Input: Vec of (user_id, balance) where positive = creditor, negative = debtor.
/// Balances must sum to zero.
/// Output: Vec of Transaction with conservation guarantee.
pub fn compute_from_balances(
    user_ids: &Vec<Vec<u8>>,
    balances: &Vec<i64>,
) -> (transactions: Vec<Transaction>)
    requires
        user_ids.len() == balances.len(),
        spec::settle_requires(balances@),
    ensures
        spec::settle_ensures(balances@, transactions@.map(|_i, t: Transaction| t@)),
{
    // Separate into creditors and debtors.
    let mut creditor_ids: Vec<Vec<u8>> = Vec::new();
    let mut creditor_amounts: Vec<i64> = Vec::new();
    let mut debtor_ids: Vec<Vec<u8>> = Vec::new();
    let mut debtor_amounts: Vec<i64> = Vec::new();

    let mut i: usize = 0;
    while i < balances.len()
        invariant
            i <= balances.len(),
            creditor_ids.len() == creditor_amounts.len(),
            debtor_ids.len() == debtor_amounts.len(),
        decreases balances.len() - i,
    {
        if balances[i] > 0 {
            creditor_ids.push(user_ids[i].clone());
            creditor_amounts.push(balances[i]);
        } else if balances[i] < 0 {
            debtor_ids.push(user_ids[i].clone());
            debtor_amounts.push(-balances[i]);
        }
        i = i + 1;
    }

    // Greedy matching.
    let mut transactions: Vec<Transaction> = Vec::new();
    let mut ci: usize = 0;
    let mut di: usize = 0;

    while ci < creditor_amounts.len() && di < debtor_amounts.len()
        invariant
            ci <= creditor_amounts.len(),
            di <= debtor_amounts.len(),
            creditor_ids.len() == creditor_amounts.len(),
            debtor_ids.len() == debtor_amounts.len(),
        decreases
            (creditor_amounts.len() - ci) + (debtor_amounts.len() - di),
    {
        let credit = creditor_amounts[ci];
        let debt = debtor_amounts[di];

        if credit == 0 {
            ci = ci + 1;
        } else if debt == 0 {
            di = di + 1;
        } else {
            let amount = if credit <= debt { credit } else { debt };

            transactions.push(Transaction {
                from_user_id: debtor_ids[di].clone(),
                to_user_id: creditor_ids[ci].clone(),
                amount_cents: amount,
            });

            creditor_amounts.set(ci, credit - amount);
            debtor_amounts.set(di, debt - amount);

            if credit - amount == 0 {
                ci = ci + 1;
            }
            if debt - amount == 0 {
                di = di + 1;
            }
        }
    }

    transactions
}

}
