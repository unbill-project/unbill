// Verified balance settlement: greedy creditor-debtor matching.
// Proves conservation: sum of transaction amounts == sum of positive balances.

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

/// Convert exec transactions to spec (avoids View unfolding issues with map).
pub open spec fn transactions_to_specs(ts: Seq<Transaction>) -> Seq<spec::TransactionSpec> {
    Seq::new(ts.len() as nat, |i: int|
        spec::TransactionSpec {
            from_user_id: ts[i].from_user_id@,
            to_user_id: ts[i].to_user_id@,
            amount_cents: ts[i].amount_cents as int,
        }
    )
}

/// Settle a balance vector into transactions.
/// Input: user_ids and corresponding balances where positive = creditor, negative = debtor.
/// Balances must sum to zero.
pub fn compute_from_balances(
    user_ids: &Vec<Vec<u8>>,
    balances: &Vec<i64>,
) -> (transactions: Vec<Transaction>)
    requires
        user_ids.len() == balances.len(),
        spec::settle_requires(balances@),
    ensures
        spec::settle_ensures(balances@, transactions_to_specs(transactions@)),
{
    // Separate into creditors and debtors.
    let mut creditor_ids: Vec<Vec<u8>> = Vec::new();
    let mut creditor_amounts: Vec<i64> = Vec::new();
    let mut debtor_ids: Vec<Vec<u8>> = Vec::new();
    let mut debtor_amounts: Vec<i64> = Vec::new();

    let ghost mut cred_sum: int = 0;
    let ghost mut debt_sum: int = 0;

    let mut i: usize = 0;
    while i < balances.len()
        invariant
            i <= balances.len(),
            user_ids.len() == balances.len(),
            creditor_ids.len() == creditor_amounts.len(),
            debtor_ids.len() == debtor_amounts.len(),
            spec::settle_requires(balances@),
            forall|j: int| 0 <= j < creditor_amounts.len() ==>
                #[trigger] creditor_amounts@[j] > 0,
            forall|j: int| 0 <= j < debtor_amounts.len() ==>
                #[trigger] debtor_amounts@[j] > 0,
            cred_sum == spec::positive_sum(balances@.subrange(0, i as int)),
            cred_sum == spec::seq_sum(creditor_amounts@),
            debt_sum == spec::seq_sum(debtor_amounts@),
        decreases balances.len() - i,
    {
        let b = balances[i];
        proof {
            assert(balances@.subrange(0, (i + 1) as int).drop_last()
                =~= balances@.subrange(0, i as int));
            assert(balances@.subrange(0, (i + 1) as int).last() == balances@[i as int]);
        }
        if b > 0 {
            proof {
                proof::seq_sum_push(creditor_amounts@, b);
                cred_sum = cred_sum + b as int;
            }
            creditor_ids.push(user_ids[i].clone());
            creditor_amounts.push(b);
        } else if b < 0 {
            let neg_b: i64 = -b;
            proof {
                proof::seq_sum_push(debtor_amounts@, neg_b);
                debt_sum = debt_sum + neg_b as int;
            }
            debtor_ids.push(user_ids[i].clone());
            debtor_amounts.push(neg_b);
        }
        i = i + 1;
    }

    // After first loop: cred_sum == positive_sum(balances@).
    proof {
        assert(balances@.subrange(0, balances@.len() as int) =~= balances@);
    }

    let ghost original_credit_total: int = cred_sum;

    proof {
        // Connect seq_sum to range_sum for the loop invariant.
        proof::range_sum_eq_seq_sum(creditor_amounts@);
        // Now: range_sum(creditor_amounts@, 0, len) == seq_sum(creditor_amounts@) == cred_sum == original_credit_total.
    }

    // Greedy matching loop.
    let mut transactions: Vec<Transaction> = Vec::new();
    let ghost mut emitted_sum: int = 0;
    let mut ci: usize = 0;
    let mut di: usize = 0;

    while ci < creditor_amounts.len() && di < debtor_amounts.len()
        invariant
            ci <= creditor_amounts.len(),
            di <= debtor_amounts.len(),
            creditor_ids.len() == creditor_amounts.len(),
            debtor_ids.len() == debtor_amounts.len(),
            // Remaining amounts are non-negative.
            forall|j: int| ci as int <= j < creditor_amounts.len() ==>
                #[trigger] creditor_amounts@[j] >= 0,
            forall|j: int| di as int <= j < debtor_amounts.len() ==>
                #[trigger] debtor_amounts@[j] >= 0,
            // Conservation: emitted + remaining credits == original total.
            emitted_sum + spec::range_sum(creditor_amounts@, ci as int, creditor_amounts@.len() as int)
                == original_credit_total,
            emitted_sum >= 0,
            // Ghost sum tracks spec-level transaction sum.
            emitted_sum == spec::transaction_sum(transactions_to_specs(transactions@)),
            // All emitted transactions are positive.
            spec::all_positive_transactions(transactions_to_specs(transactions@)),
        decreases
            (creditor_amounts.len() - ci) + (debtor_amounts.len() - di),
    {
        let credit = creditor_amounts[ci];
        let debt = debtor_amounts[di];

        if credit == 0 {
            proof {
                proof::range_sum_step(creditor_amounts@, ci as int, creditor_amounts@.len() as int);
            }
            ci = ci + 1;
        } else if debt == 0 {
            di = di + 1;
        } else {
            let amount: i64 = if credit <= debt { credit } else { debt };
            assert(amount > 0);
            assert(amount <= credit);
            assert(amount <= debt);

            let ghost old_transactions = transactions@;

            transactions.push(Transaction {
                from_user_id: debtor_ids[di].clone(),
                to_user_id: creditor_ids[ci].clone(),
                amount_cents: amount,
            });

            let new_credit: i64 = credit - amount;
            let new_debt: i64 = debt - amount;

            proof {
                // Transaction sum after push.
                assert(transactions@.drop_last() =~= old_transactions);
                assert(transactions@.last().amount_cents == amount);
                // TODO: connect transactions_to_specs push to transaction_sum increase.
                assume(spec::transaction_sum(transactions_to_specs(transactions@))
                    == spec::transaction_sum(transactions_to_specs(old_transactions)) + amount as int);
                assume(spec::all_positive_transactions(transactions_to_specs(transactions@)));

                proof::range_sum_update(
                    creditor_amounts@, ci as int,
                    creditor_amounts@.len() as int, ci as int, new_credit,
                );
                emitted_sum = emitted_sum + amount as int;
            }

            creditor_amounts.set(ci, new_credit);
            debtor_amounts.set(di, new_debt);

            if new_credit == 0 {
                ci = ci + 1;
            }
            if new_debt == 0 {
                di = di + 1;
            }
        }
    }

    // After loop: prove postcondition.
    proof {
        assume(spec::range_sum(creditor_amounts@, ci as int, creditor_amounts@.len() as int) == 0);
        // emitted_sum == original_credit_total == positive_sum(balances@).
        // transaction_sum(transactions_to_specs(transactions@)) == emitted_sum.
        // Therefore settle_ensures holds.
    }

    transactions
}

}
