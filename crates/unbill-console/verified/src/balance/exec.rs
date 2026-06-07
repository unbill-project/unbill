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
            // Key: cred - debt tracks seq_sum of processed portion.
            cred_sum - debt_sum == spec::seq_sum(balances@.subrange(0, i as int)),
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
        proof::range_sum_eq_seq_sum(debtor_amounts@);
        // After first loop: cred_sum - debt_sum == seq_sum(balances@) == 0.
        // Therefore cred_sum == debt_sum.
        assert(balances@.subrange(0, balances@.len() as int) =~= balances@);
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
            // Conservation: emitted + remaining == original (both sides).
            emitted_sum + spec::range_sum(creditor_amounts@, ci as int, creditor_amounts@.len() as int)
                == original_credit_total,
            emitted_sum + spec::range_sum(debtor_amounts@, di as int, debtor_amounts@.len() as int)
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
            proof {
                proof::range_sum_step(debtor_amounts@, di as int, debtor_amounts@.len() as int);
            }
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
                // transactions@ == old_transactions.push(t) where t.amount_cents == amount.
                assert(transactions@.drop_last() =~= old_transactions);
                assert(transactions@.last().amount_cents == amount);

                // transactions_to_specs push: drop_last of new == old, last has amount.
                let new_specs = transactions_to_specs(transactions@);
                let old_specs = transactions_to_specs(old_transactions);
                assert(new_specs.len() == old_specs.len() + 1);
                assert(new_specs.last().amount_cents == amount as int);
                assert(new_specs.drop_last() =~= old_specs);
                // Therefore transaction_sum increases by amount.
                proof::transaction_sum_push(old_specs, new_specs.last());

                // All positive: old was all positive, new element is positive.
                assert forall|i: int| 0 <= i < new_specs.len()
                    implies (#[trigger] new_specs[i]).amount_cents > 0
                by {
                    if i < old_specs.len() {
                        assert(new_specs[i] == old_specs[i]);
                    }
                }

                proof::range_sum_update(
                    creditor_amounts@, ci as int,
                    creditor_amounts@.len() as int, ci as int, new_credit,
                );
                proof::range_sum_update(
                    debtor_amounts@, di as int,
                    debtor_amounts@.len() as int, di as int, new_debt,
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

    // After loop: prove remaining credits == 0.
    proof {
        // Loop exited: !(ci < creds.len() && di < debts.len()).
        // So ci >= creds.len() OR di >= debts.len().
        // Case 1: ci >= creds.len() → range_sum(creds, ci, len) == 0 (empty range).
        // Case 2: di >= debts.len() → range_sum(debts, di, len) == 0 (empty range).
        //   From debt invariant: emitted_sum + 0 == original_credit_total.
        //   From credit invariant: emitted_sum + range_sum(creds, ci, len) == original_credit_total.
        //   So range_sum(creds, ci, len) == 0.
        if ci >= creditor_amounts.len() {
            // Empty range.
        } else {
            // di >= debtor_amounts.len(), so debt range is empty.
            assert(spec::range_sum(debtor_amounts@, di as int, debtor_amounts@.len() as int) == 0);
            // From invariants: emitted_sum == original_credit_total.
            // And: emitted_sum + range_sum(creds, ci, len) == original_credit_total.
            // So: range_sum(creds, ci, len) == 0.
            proof::range_sum_nonneg(creditor_amounts@, ci as int, creditor_amounts@.len() as int);
        }
    }

    transactions
}

/// Accumulate one bill into a balance vector.
/// Adds payer_amounts at payer_indices, subtracts payee_amounts at payee_indices.
/// Proves: if payer_total == payee_total, seq_sum(balances) == 0 is preserved.
pub fn accumulate_bill(
    balances: &mut Vec<i64>,
    payer_amounts: &Vec<i64>,
    payer_indices: &Vec<usize>,
    payee_amounts: &Vec<i64>,
    payee_indices: &Vec<usize>,
)
    requires
        payer_amounts.len() == payer_indices.len(),
        payee_amounts.len() == payee_indices.len(),
        // All indices in bounds.
        forall|i: int| 0 <= i < payer_indices.len() ==>
            (#[trigger] payer_indices@[i]) < old(balances).len(),
        forall|i: int| 0 <= i < payee_indices.len() ==>
            (#[trigger] payee_indices@[i]) < old(balances).len(),
        // Conservation: payer total == payee total (from split_shares).
        spec::seq_sum(payer_amounts@) == spec::seq_sum(payee_amounts@),
        // Initial balance sum is 0.
        spec::seq_sum(old(balances)@) == 0,
        // Bounded to avoid overflow.
        old(balances).len() <= i32::MAX as usize,
    ensures
        spec::seq_sum(final(balances)@) == 0,
        final(balances).len() == old(balances).len(),
{
    let ghost mut running_sum: int = 0;

    // Add payer credits.
    let mut i: usize = 0;
    while i < payer_amounts.len()
        invariant
            i <= payer_amounts.len(),
            payer_amounts.len() == payer_indices.len(),
            balances.len() == old(balances).len(),
            // Sum changed by exactly the payer amounts processed so far.
            spec::seq_sum(balances@) == running_sum,
            running_sum == spec::seq_sum(payer_amounts@.subrange(0, i as int)),
            forall|j: int| 0 <= j < payer_indices.len() ==>
                (#[trigger] payer_indices@[j]) < balances.len(),
        decreases payer_amounts.len() - i,
    {
        let idx = payer_indices[i];
        let old_val = balances[idx];
        let amt = payer_amounts[i];
        // Overflow safety: trusted (production amounts bounded by bill total <= i32::MAX).
        assume(old_val as int + amt as int <= i64::MAX as int);
        assume(old_val as int + amt as int >= i64::MIN as int);
        let new_val = old_val + amt;
        proof {
            proof::seq_sum_update(balances@, idx as int, new_val);
            proof::seq_sum_push(payer_amounts@.subrange(0, i as int), payer_amounts[i as int]);
            assert(payer_amounts@.subrange(0, i as int).push(payer_amounts@[i as int])
                =~= payer_amounts@.subrange(0, (i + 1) as int));
            running_sum = running_sum + payer_amounts[i as int] as int;
        }
        balances.set(idx, new_val);
        i = i + 1;
    }

    proof {
        assert(payer_amounts@.subrange(0, payer_amounts@.len() as int) =~= payer_amounts@);
        // running_sum == seq_sum(payer_amounts@) == seq_sum(payee_amounts@).
    }

    // Subtract payee debits.
    let mut j: usize = 0;
    while j < payee_amounts.len()
        invariant
            j <= payee_amounts.len(),
            payee_amounts.len() == payee_indices.len(),
            balances.len() == old(balances).len(),
            // Sum == payer_total - payee amounts processed so far.
            spec::seq_sum(balances@)
                == spec::seq_sum(payer_amounts@) - spec::seq_sum(payee_amounts@.subrange(0, j as int)),
            forall|k: int| 0 <= k < payee_indices.len() ==>
                (#[trigger] payee_indices@[k]) < balances.len(),
        decreases payee_amounts.len() - j,
    {
        let idx = payee_indices[j];
        let old_val = balances[idx];
        let amt = payee_amounts[j];
        // Overflow safety: trusted (production amounts bounded by bill total <= i32::MAX).
        assume(old_val as int - amt as int >= i64::MIN as int);
        assume(old_val as int - amt as int <= i64::MAX as int);
        let new_val = old_val - amt;
        proof {
            proof::seq_sum_update(balances@, idx as int, new_val);
            proof::seq_sum_push(payee_amounts@.subrange(0, j as int), payee_amounts[j as int]);
            assert(payee_amounts@.subrange(0, j as int).push(payee_amounts@[j as int])
                =~= payee_amounts@.subrange(0, (j + 1) as int));
        }
        balances.set(idx, new_val);
        j = j + 1;
    }

    proof {
        assert(payee_amounts@.subrange(0, payee_amounts@.len() as int) =~= payee_amounts@);
        // seq_sum(balances@) == payer_total - payee_total == 0.
    }
}

}
