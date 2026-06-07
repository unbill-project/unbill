// Verified balance settlement: greedy creditor-debtor matching.
// Proves conservation: sum of transaction amounts == sum of positive balances.

use super::proof;
use super::spec;
use unbill_model_verified::ledger::exec::{Bill, LedgerState, User};
use vstd::prelude::*;
use vstd::slice::SliceAdditionalExecFns;

verus! {

/// A settlement transaction (runtime).
pub struct Transaction {
    pub from_user_id: u128,
    pub to_user_id: u128,
    pub amount_cents: i64,
}

impl View for Transaction {
    type V = spec::TransactionSpec;
    open spec fn view(&self) -> spec::TransactionSpec {
        spec::TransactionSpec {
            from_user_id: self.from_user_id,
            to_user_id: self.to_user_id,
            amount_cents: self.amount_cents as int,
        }
    }
}

/// Convert exec transactions to spec.
pub open spec fn transactions_to_specs(ts: Seq<Transaction>) -> Seq<spec::TransactionSpec> {
    Seq::new(ts.len() as nat, |i: int|
        spec::TransactionSpec {
            from_user_id: ts[i].from_user_id,
            to_user_id: ts[i].to_user_id,
            amount_cents: ts[i].amount_cents as int,
        }
    )
}

/// Settle a balance vector into transactions.
/// Input: user_ids and corresponding balances where positive = creditor, negative = debtor.
/// Balances must sum to zero.
pub fn compute_from_balances(
    user_ids: &Vec<u128>,
    balances: &Vec<i64>,
) -> (transactions: Vec<Transaction>)
    requires
        user_ids.len() == balances.len(),
        spec::settle_requires(balances@),
    ensures
        spec::settle_ensures(balances@, transactions_to_specs(transactions@)),
{
    // Separate into creditors and debtors.
    let mut creditor_ids: Vec<u128> = Vec::new();
    let mut creditor_amounts: Vec<i64> = Vec::new();
    let mut debtor_ids: Vec<u128> = Vec::new();
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
            creditor_ids.push(user_ids[i]);
            creditor_amounts.push(b);
        } else if b < 0 {
            let neg_b: i64 = -b;
            proof {
                proof::seq_sum_push(debtor_amounts@, neg_b);
                debt_sum = debt_sum + neg_b as int;
            }
            debtor_ids.push(user_ids[i]);
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
                from_user_id: debtor_ids[di],
                to_user_id: creditor_ids[ci],
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

/// Compute per-user balances from a ledger.
/// Takes a LedgerState (from the verified model), returns balances indexed by user position.
/// balances[i] = net balance for ledger.users[i].
/// Proves: seq_sum(result) == 0 (conservation across all bills).
pub fn compute_balances(
    ledger: &LedgerState,
) -> (balances: Vec<i64>)
    requires
        // Ledger invariant holds (bills are well-formed, users exist, etc.).
        unbill_model_verified::ledger::spec::ledger_invariant(ledger@),
        // split_shares preconditions for every bill.
        forall|b: int| 0 <= b < ledger.bills.len() ==> (
            unbill_model_verified::ledger::spec::split_shares_requires(
                unbill_model_verified::ledger::exec::shares_to_specs((#[trigger] ledger.bills@[b]).payers@),
                ledger.bills@[b].amount_cents,
            )
            && unbill_model_verified::ledger::spec::split_shares_requires(
                unbill_model_verified::ledger::exec::shares_to_specs(ledger.bills@[b].payees@),
                ledger.bills@[b].amount_cents,
            )
        ),
        // Exec-level user existence: each share's user_id exists in ledger.users.
        forall|b: int, s: int| #![trigger ledger.bills@[b].payers@[s]]
            0 <= b < ledger.bills.len() && 0 <= s < ledger.bills@[b].payers.len() ==>
            exists|k: int| 0 <= k < ledger.users.len()
                && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[b].payers@[s].user_id,
        forall|b: int, s: int| #![trigger ledger.bills@[b].payees@[s]]
            0 <= b < ledger.bills.len() && 0 <= s < ledger.bills@[b].payees.len() ==>
            exists|k: int| 0 <= k < ledger.users.len()
                && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[b].payees@[s].user_id,
        // Overflow bound: total amount across all bills fits in i64/4.
        // (Guarantees no individual balance exceeds i64/2 during accumulation.)
        ledger.bills.len() <= i32::MAX as usize,
        forall|b: int| 0 <= b < ledger.bills.len() ==>
            (#[trigger] ledger.bills@[b]).amount_cents >= 0
            && ledger.bills@[b].amount_cents <= i32::MAX as i64,
    ensures
        balances.len() == ledger.users.len(),
        spec::seq_sum(balances@) == 0,
{
    // Initialize balances to 0 for each user.
    let mut balances: Vec<i64> = Vec::new();
    let mut u: usize = 0;
    while u < ledger.users.len()
        invariant
            u <= ledger.users.len(),
            balances.len() == u,
            spec::seq_sum(balances@) == 0,
            forall|k: int| 0 <= k < balances.len() ==>
                (#[trigger] balances@[k]) > -4611686018427387903i64
                && balances@[k] < 4611686018427387903i64,
        decreases ledger.users.len() - u,
    {
        proof { proof::seq_sum_push(balances@, 0i64); }
        balances.push(0i64);
        u = u + 1;
    }

    // Process each bill.
    let mut b: usize = 0;
    while b < ledger.bills.len()
        invariant
            b <= ledger.bills.len(),
            balances.len() == ledger.users.len(),
            spec::seq_sum(balances@) == 0,
            unbill_model_verified::ledger::spec::ledger_invariant(ledger@),
            forall|k: int| 0 <= k < ledger.bills.len() ==> (
                unbill_model_verified::ledger::spec::split_shares_requires(
                    unbill_model_verified::ledger::exec::shares_to_specs((#[trigger] ledger.bills@[k]).payers@),
                    ledger.bills@[k].amount_cents,
                )
                && unbill_model_verified::ledger::spec::split_shares_requires(
                    unbill_model_verified::ledger::exec::shares_to_specs(ledger.bills@[k].payees@),
                    ledger.bills@[k].amount_cents,
                )
            ),
            forall|k: int, s: int| #![trigger ledger.bills@[k].payers@[s]]
                0 <= k < ledger.bills.len() && 0 <= s < ledger.bills@[k].payers.len() ==>
                exists|j: int| 0 <= j < ledger.users.len()
                    && (#[trigger] ledger.users@[j]).user_id == ledger.bills@[k].payers@[s].user_id,
            forall|k: int, s: int| #![trigger ledger.bills@[k].payees@[s]]
                0 <= k < ledger.bills.len() && 0 <= s < ledger.bills@[k].payees.len() ==>
                exists|j: int| 0 <= j < ledger.users.len()
                    && (#[trigger] ledger.users@[j]).user_id == ledger.bills@[k].payees@[s].user_id,
            ledger.bills.len() <= i32::MAX as usize,
            forall|k: int| 0 <= k < ledger.bills.len() ==>
                (#[trigger] ledger.bills@[k]).amount_cents >= 0
                && ledger.bills@[k].amount_cents <= i32::MAX as i64,
            // Balance bound: within i64/2 (generous; maintained since each op changes by at most i32::MAX).
            forall|k: int| 0 <= k < balances.len() ==>
                (#[trigger] balances@[k]) > i64::MIN / 2
                && balances@[k] < i64::MAX / 2,
        decreases ledger.bills.len() - b,
    {
        let bill = &ledger.bills[b];

        // Call verified split_shares for payers and payees.
        // remainder_idx = 0; bound trivially satisfied since len <= usize::MAX.
        assert(0usize <= usize::MAX - bill.payers.len());
        assert(0usize <= usize::MAX - bill.payees.len());
        let payer_amounts = crate::settlement::split_shares(
            &bill.payers, bill.amount_cents, 0,
        );
        let payee_amounts = crate::settlement::split_shares(
            &bill.payees, bill.amount_cents, 0,
        );

        // Bridge: split_shares ensures amount_sum == amount_cents.
        // Connect to seq_sum for our invariant.
        proof {
            proof::amount_sum_eq_seq_sum(payer_amounts@);
            proof::amount_sum_eq_seq_sum(payee_amounts@);
        }

        // Add payer credits to balances.
        let mut i: usize = 0;
        while i < payer_amounts.len()
            invariant
                i <= payer_amounts.len(),
                payer_amounts.len() == bill.payers.len(),
                balances.len() == ledger.users.len(),
                spec::seq_sum(balances@) == spec::seq_sum(payer_amounts@.subrange(0, i as int)),
                b < ledger.bills.len(),
                ledger.bills.len() <= i32::MAX as usize,
                forall|s: int| #![trigger bill.payers@[s]]
                    0 <= s < bill.payers.len() ==>
                    exists|k: int| 0 <= k < ledger.users.len()
                        && (#[trigger] ledger.users@[k]).user_id == bill.payers@[s].user_id,
                // Amounts non-negative (from split_shares ensures).
                forall|s: int| 0 <= s < payer_amounts.len() ==> (#[trigger] payer_amounts@[s]) >= 0,
                forall|k: int| 0 <= k < balances.len() ==> (
                    #[trigger] balances@[k] > -4611686018427387903i64
                    && balances@[k] < 4611686018427387903i64
                ),
            decreases payer_amounts.len() - i,
        {
            // User exists: from precondition (exec-level user existence).
            let user_idx = find_user_index(&ledger.users, bill.payers[i].user_id);
            let old_val = balances[user_idx];
            let amt = payer_amounts[i];
            // Overflow: old_val within i64/2 and amt <= i32::MAX.
            assert(old_val as int + amt as int <= i64::MAX as int) by(nonlinear_arith)
                requires old_val < 4611686018427387903i64, amt >= 0i64;
            assert(old_val as int + amt as int >= i64::MIN as int) by(nonlinear_arith)
                requires old_val > -4611686018427387903i64, amt >= 0i64;
            let new_val = old_val + amt;
            proof {
                proof::seq_sum_update(balances@, user_idx as int, new_val);
                proof::seq_sum_push(payer_amounts@.subrange(0, i as int), payer_amounts@[i as int]);
                assert(payer_amounts@.subrange(0, i as int).push(payer_amounts@[i as int])
                    =~= payer_amounts@.subrange(0, (i + 1) as int));
            }
            balances.set(user_idx, new_val);
            i = i + 1;
        }
        proof {
            assert(payer_amounts@.subrange(0, payer_amounts@.len() as int) =~= payer_amounts@);
        }

        // Subtract payee debits from balances.
        let mut j: usize = 0;
        while j < payee_amounts.len()
            invariant
                j <= payee_amounts.len(),
                payee_amounts.len() == bill.payees.len(),
                balances.len() == ledger.users.len(),
                spec::seq_sum(balances@)
                    == spec::seq_sum(payer_amounts@) - spec::seq_sum(payee_amounts@.subrange(0, j as int)),
                b < ledger.bills.len(),
                ledger.bills.len() <= i32::MAX as usize,
                forall|s: int| #![trigger bill.payees@[s]]
                    0 <= s < bill.payees.len() ==>
                    exists|k: int| 0 <= k < ledger.users.len()
                        && (#[trigger] ledger.users@[k]).user_id == bill.payees@[s].user_id,
                forall|s: int| 0 <= s < payee_amounts.len() ==> (#[trigger] payee_amounts@[s]) >= 0,
                forall|k: int| 0 <= k < balances.len() ==> (
                    #[trigger] balances@[k] > -4611686018427387903i64
                    && balances@[k] < 4611686018427387903i64
                ),
            decreases payee_amounts.len() - j,
        {
            let user_idx = find_user_index(&ledger.users, bill.payees[j].user_id);
            let old_val = balances[user_idx];
            let amt = payee_amounts[j];
            assert(old_val as int - amt as int >= i64::MIN as int) by(nonlinear_arith)
                requires old_val > -4611686018427387903i64, amt >= 0i64, amt <= i32::MAX as i64;
            assert(old_val as int - amt as int <= i64::MAX as int) by(nonlinear_arith)
                requires old_val < 4611686018427387903i64, amt >= 0i64;
            let new_val = old_val - amt;
            proof {
                proof::seq_sum_update(balances@, user_idx as int, new_val);
                proof::seq_sum_push(payee_amounts@.subrange(0, j as int), payee_amounts@[j as int]);
                assert(payee_amounts@.subrange(0, j as int).push(payee_amounts@[j as int])
                    =~= payee_amounts@.subrange(0, (j + 1) as int));
            }
            balances.set(user_idx, new_val);
            j = j + 1;
        }
        proof {
            assert(payee_amounts@.subrange(0, payee_amounts@.len() as int) =~= payee_amounts@);
        }

        b = b + 1;
    }

    balances
}

/// Find the index of a user_id in the users list.
/// Requires: the user exists (guaranteed by ledger_invariant + bill_well_formed).
fn find_user_index(users: &Vec<User>, user_id: u128) -> (idx: usize)
    requires
        exists|i: int| 0 <= i < users.len() && (#[trigger] users@[i]).user_id == user_id,
    ensures
        idx < users.len(),
{
    let mut k: usize = 0;
    while k < users.len()
        invariant
            k <= users.len(),
            forall|j: int| 0 <= j < k ==> (#[trigger] users@[j]).user_id != user_id,
            exists|i: int| k <= i < users.len() && (#[trigger] users@[i]).user_id == user_id,
        decreases users.len() - k,
    {
        if users[k].user_id == user_id {
            return k;
        }
        k = k + 1;
    }
    proof { assert(false); }
    0
}

}
