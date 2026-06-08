// Verified balance settlement: greedy creditor-debtor matching.
// Proves conservation: sum of transaction amounts == sum of positive balances.

use super::proof;
use super::spec;
use unbill_model_verified::ledger::exec::*;
use unbill_model_verified::ledger::spec::*;
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

    proof {
        assert(balances@.subrange(0, balances@.len() as int) =~= balances@);
    }

    let ghost original_credit_total: int = cred_sum;

    proof {
        proof::range_sum_eq_seq_sum(creditor_amounts@);
        proof::range_sum_eq_seq_sum(debtor_amounts@);
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
            forall|j: int| ci as int <= j < creditor_amounts.len() ==>
                #[trigger] creditor_amounts@[j] >= 0,
            forall|j: int| di as int <= j < debtor_amounts.len() ==>
                #[trigger] debtor_amounts@[j] >= 0,
            emitted_sum + spec::range_sum(creditor_amounts@, ci as int, creditor_amounts@.len() as int)
                == original_credit_total,
            emitted_sum + spec::range_sum(debtor_amounts@, di as int, debtor_amounts@.len() as int)
                == original_credit_total,
            emitted_sum >= 0,
            emitted_sum == spec::transaction_sum(transactions_to_specs(transactions@)),
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
                assert(transactions@.drop_last() =~= old_transactions);
                assert(transactions@.last().amount_cents == amount);

                let new_specs = transactions_to_specs(transactions@);
                let old_specs = transactions_to_specs(old_transactions);
                assert(new_specs.len() == old_specs.len() + 1);
                assert(new_specs.last().amount_cents == amount as int);
                assert(new_specs.drop_last() =~= old_specs);
                proof::transaction_sum_push(old_specs, new_specs.last());

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

    proof {
        if ci >= creditor_amounts.len() {
        } else {
            assert(spec::range_sum(debtor_amounts@, di as int, debtor_amounts@.len() as int) == 0);
            proof::range_sum_nonneg(creditor_amounts@, ci as int, creditor_amounts@.len() as int);
        }
    }

    transactions
}

// ---------------------------------------------------------------------------
// Balance accumulation
// ---------------------------------------------------------------------------

/// Balance bound predicate: |balance| < n * bal_M().
/// Used as an invariant: after processing n bills, each balance satisfies this.
pub open spec fn bal_bounded(bal: i64, n: int) -> bool {
    bal as int > -(n * spec::bal_M()) && (bal as int) < n * spec::bal_M()
}

/// Compute per-user balances from effective bills in a ledger.
/// Takes effective bill indices and remainder indices.
/// Proves: seq_sum(result) == 0 (conservation).
pub fn compute_balances(
    ledger: &LedgerState,
    effective_indices: &Vec<usize>,
    remainder_indices: &Vec<usize>,
) -> (balances: Vec<i64>)
    requires
        unbill_model_verified::ledger::spec::ledger_invariant(ledger@),
        forall|i: int| 0 <= i < effective_indices.len() ==>
            (#[trigger] effective_indices@[i]) < ledger.bills.len(),
        effective_indices.len() <= ledger.bills.len(),
        forall|bi: int| 0 <= bi < ledger.bills.len() ==> (
            unbill_model_verified::ledger::spec::split_shares_requires(
                unbill_model_verified::ledger::exec::shares_to_specs((#[trigger] ledger.bills@[bi]).payers@),
                ledger.bills@[bi].amount_cents,
            )
            && unbill_model_verified::ledger::spec::split_shares_requires(
                unbill_model_verified::ledger::exec::shares_to_specs(ledger.bills@[bi].payees@),
                ledger.bills@[bi].amount_cents,
            )
        ),
        forall|bi: int, s: int| #![trigger ledger.bills@[bi].payers@[s]]
            0 <= bi < ledger.bills.len() && 0 <= s < ledger.bills@[bi].payers.len() ==>
            exists|k: int| 0 <= k < ledger.users.len()
                && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[bi].payers@[s].user_id,
        forall|bi: int, s: int| #![trigger ledger.bills@[bi].payees@[s]]
            0 <= bi < ledger.bills.len() && 0 <= s < ledger.bills@[bi].payees.len() ==>
            exists|k: int| 0 <= k < ledger.users.len()
                && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[bi].payees@[s].user_id,
        ledger.bills.len() <= i32::MAX as usize,
        forall|bi: int| 0 <= bi < ledger.bills.len() ==>
            (#[trigger] ledger.bills@[bi]).amount_cents >= 0
            && ledger.bills@[bi].amount_cents <= i32::MAX as i64,
        remainder_indices.len() == ledger.bills.len(),
        forall|i: int| 0 <= i < remainder_indices.len() ==>
            (#[trigger] remainder_indices@[i]) <= usize::MAX - (i32::MAX as usize),
    ensures
        balances.len() == ledger.users.len(),
        spec::seq_sum(balances@) == 0,
        forall|k: int| 0 <= k < balances.len() ==>
            (#[trigger] balances@[k]) > i64::MIN && balances@[k] < i64::MAX,
{
    // Initialize balances to 0 for each user.
    let mut balances: Vec<i64> = Vec::new();
    let mut u: usize = 0;
    while u < ledger.users.len()
        invariant
            u <= ledger.users.len(),
            balances.len() == u,
            spec::seq_sum(balances@) == 0,
            forall|k: int| 0 <= k < u ==> (#[trigger] balances@[k]) == 0i64,
        decreases ledger.users.len() - u,
    {
        proof { proof::seq_sum_push(balances@, 0i64); }
        balances.push(0i64);
        u = u + 1;
    }

    // Process each effective bill.
    // Growing bound: after b bills, |balance[k]| < (b+1) * bal_M().
    let mut b: usize = 0;
    while b < effective_indices.len()
        invariant
            b <= effective_indices.len(),
            balances.len() == ledger.users.len(),
            spec::seq_sum(balances@) == 0,
            unbill_model_verified::ledger::spec::ledger_invariant(ledger@),
            forall|i: int| 0 <= i < effective_indices.len() ==>
                (#[trigger] effective_indices@[i]) < ledger.bills.len(),
            effective_indices.len() <= ledger.bills.len(),
            forall|bi: int| 0 <= bi < ledger.bills.len() ==> (
                unbill_model_verified::ledger::spec::split_shares_requires(
                    unbill_model_verified::ledger::exec::shares_to_specs((#[trigger] ledger.bills@[bi]).payers@),
                    ledger.bills@[bi].amount_cents,
                )
                && unbill_model_verified::ledger::spec::split_shares_requires(
                    unbill_model_verified::ledger::exec::shares_to_specs(ledger.bills@[bi].payees@),
                    ledger.bills@[bi].amount_cents,
                )
            ),
            forall|bi: int, s: int| #![trigger ledger.bills@[bi].payers@[s]]
                0 <= bi < ledger.bills.len() && 0 <= s < ledger.bills@[bi].payers.len() ==>
                exists|k: int| 0 <= k < ledger.users.len()
                    && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[bi].payers@[s].user_id,
            forall|bi: int, s: int| #![trigger ledger.bills@[bi].payees@[s]]
                0 <= bi < ledger.bills.len() && 0 <= s < ledger.bills@[bi].payees.len() ==>
                exists|k: int| 0 <= k < ledger.users.len()
                    && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[bi].payees@[s].user_id,
            ledger.bills.len() <= i32::MAX as usize,
            forall|bi: int| 0 <= bi < ledger.bills.len() ==>
                (#[trigger] ledger.bills@[bi]).amount_cents >= 0
                && ledger.bills@[bi].amount_cents <= i32::MAX as i64,
            remainder_indices.len() == ledger.bills.len(),
            forall|i: int| 0 <= i < remainder_indices.len() ==>
                (#[trigger] remainder_indices@[i]) <= usize::MAX - (i32::MAX as usize),
            // Growing balance bound.
            forall|k: int| 0 <= k < balances.len() ==>
                bal_bounded(#[trigger] balances@[k], b as int + 1),
        decreases effective_indices.len() - b,
    {
        let bill_idx: usize = effective_indices[b];
        let bill = &ledger.bills[bill_idx];
        let amount = bill.amount_cents;
        // Pre-compute the current balance bound for overflow proofs.
        // bnd = (b+1) * bal_M(). Overflow assertions use bnd instead of spec::bal_M() * (b+1).
        let ghost bnd: int = spec::bal_M() * (b as int + 1);

        let rem_idx = remainder_indices[bill_idx];
        assert(rem_idx <= usize::MAX - bill.payers.len());
        assert(rem_idx <= usize::MAX - bill.payees.len());
        let payer_amounts = crate::settlement::split_shares(
            &bill.payers, amount, rem_idx,
        );
        let payee_amounts = crate::settlement::split_shares(
            &bill.payees, amount, rem_idx,
        );

        // Ghost: snapshot balances before payer processing.
        let ghost snapshot = balances@;

        // Add payer credits to balances.
        let mut i: usize = 0;
        while i < payer_amounts.len()
            invariant
                i <= payer_amounts.len(),
                payer_amounts.len() == bill.payers.len(),
                balances.len() == ledger.users.len(),
                spec::seq_sum(balances@) == spec::seq_sum(snapshot)
                    + spec::seq_sum(payer_amounts@.subrange(0, i as int)),
                b < effective_indices.len(),
                b as int + 1 <= i32::MAX as int,
                bill_idx < ledger.bills.len(),
                ledger.bills.len() <= i32::MAX as usize,
                amount == bill.amount_cents,
                amount >= 0,
                amount <= i32::MAX as i64,
                forall|s: int| 0 <= s < payer_amounts.len() ==>
                    (#[trigger] payer_amounts@[s]) >= 0 && payer_amounts@[s] <= amount,
                spec::seq_sum(payer_amounts@) == amount as int,
                forall|s: int| #![trigger bill.payers@[s]]
                    0 <= s < bill.payers.len() ==>
                    exists|k: int| 0 <= k < ledger.users.len()
                        && (#[trigger] ledger.users@[k]).user_id == bill.payers@[s].user_id,
                // Snapshot is fixed; only adds since then.
                snapshot.len() == balances@.len(),
                forall|k: int| 0 <= k < balances.len() ==>
                    (#[trigger] balances@[k]) >= snapshot[k],
                // Snapshot bounds from outer growing bound.
                forall|k: int| 0 <= k < snapshot.len() ==>
                    bal_bounded(#[trigger] snapshot[k], b as int + 1),
                bnd == spec::bal_M() * (b as int + 1),
            decreases payer_amounts.len() - i,
        {
            let user_idx = find_user_index(&ledger.users, bill.payers[i].user_id);
            let old_val = balances[user_idx];
            let amt = payer_amounts[i];

            // Overflow safety via snapshot bound lemma.
            proof {
                proof::partial_sum_le_total(payer_amounts@, i as int);
                proof::bound_elem_from_nonneg_increase(snapshot, balances@, amount as int);
                // Trigger quantifier for user_idx:
                let snap_raw = snapshot[user_idx as int];
                // bal_bounded(snap_raw, b+1) unfolds to:
                //   snap_raw as int > -((b+1) * bal_M()) && snap_raw as int < (b+1) * bal_M()
                // bnd == bal_M() * (b+1) == (b+1) * bal_M() by commutativity
                assert(bnd == (b as int + 1) * spec::bal_M()) by(nonlinear_arith)
                    requires bnd == spec::bal_M() * (b as int + 1);
                // Prove bnd is bounded: bnd = (b+1)*bal_M() <= i32::MAX*bal_M()
                assert(bnd <= i32::MAX as int * spec::bal_M()) by(nonlinear_arith)
                    requires bnd == (b as int + 1) * spec::bal_M(),
                             b as int + 1 <= i32::MAX as int;
            }
            assert(old_val as int + amt as int <= i64::MAX as int) by(nonlinear_arith)
                requires old_val <= bnd + i32::MAX as int,
                         amt >= 0i64, amt <= i32::MAX as i64,
                         bnd <= i32::MAX as int * (i32::MAX as int + 1);
            assert(old_val as int + amt as int >= i64::MIN as int) by(nonlinear_arith)
                requires old_val >= -bnd, amt >= 0i64;

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

        // Establish bounds on snapshot_payee.
        proof {
            proof::bound_elem_from_nonneg_increase(snapshot, balances@, amount as int);
        }
        let ghost snapshot_payee = balances@;

        // Subtract payee debits from balances.
        let mut j: usize = 0;
        while j < payee_amounts.len()
            invariant
                j <= payee_amounts.len(),
                payee_amounts.len() == bill.payees.len(),
                balances.len() == ledger.users.len(),
                spec::seq_sum(balances@) == spec::seq_sum(snapshot)
                    + spec::seq_sum(payer_amounts@)
                    - spec::seq_sum(payee_amounts@.subrange(0, j as int)),
                b < effective_indices.len(),
                b as int + 1 <= i32::MAX as int,
                bill_idx < ledger.bills.len(),
                ledger.bills.len() <= i32::MAX as usize,
                amount == bill.amount_cents,
                amount >= 0,
                amount <= i32::MAX as i64,
                spec::seq_sum(payer_amounts@) == amount as int,
                forall|s: int| 0 <= s < payee_amounts.len() ==>
                    (#[trigger] payee_amounts@[s]) >= 0 && payee_amounts@[s] <= amount,
                spec::seq_sum(payee_amounts@) == amount as int,
                forall|s: int| #![trigger bill.payees@[s]]
                    0 <= s < bill.payees.len() ==>
                    exists|k: int| 0 <= k < ledger.users.len()
                        && (#[trigger] ledger.users@[k]).user_id == bill.payees@[s].user_id,
                // Only subtracts since snapshot_payee.
                snapshot_payee.len() == balances@.len(),
                forall|k: int| 0 <= k < balances.len() ==>
                    (#[trigger] balances@[k]) <= snapshot_payee[k],
                // Snapshot_payee sum (for bound_elem_from_nonneg_increase precondition).
                spec::seq_sum(snapshot_payee) == spec::seq_sum(snapshot) + amount as int,
                // Snapshot_payee bounds.
                forall|k: int| 0 <= k < snapshot_payee.len() ==>
                    (#[trigger] snapshot_payee[k]) as int >= snapshot[k] as int
                    && (snapshot_payee[k]) as int <= snapshot[k] as int + amount as int,
                // Original snapshot bounds.
                snapshot.len() == balances@.len(),
                forall|k: int| 0 <= k < snapshot.len() ==>
                    bal_bounded(#[trigger] snapshot[k], b as int + 1),
                bnd == spec::bal_M() * (b as int + 1),
            decreases payee_amounts.len() - j,
        {
            let user_idx = find_user_index(&ledger.users, bill.payees[j].user_id);
            let old_val = balances[user_idx];
            let amt = payee_amounts[j];

            // Overflow safety via reverse snapshot bound.
            proof {
                proof::partial_sum_le_total(payee_amounts@, j as int);
                proof::bound_elem_from_nonneg_increase(balances@, snapshot_payee, amount as int);
                // Assert commutativity and bound for bnd:
                assert(bnd == (b as int + 1) * spec::bal_M()) by(nonlinear_arith)
                    requires bnd == spec::bal_M() * (b as int + 1);
                assert(bnd <= i32::MAX as int * spec::bal_M()) by(nonlinear_arith)
                    requires bnd == (b as int + 1) * spec::bal_M(),
                             b as int + 1 <= i32::MAX as int;
                // Trigger quantifiers for user_idx:
                let sp_raw = snapshot_payee[user_idx as int];
                let snap_raw = snapshot[user_idx as int];
                let snap_val: int = snap_raw as int;
                // bal_bounded(snap_raw, b+1) → snap_val > -(b+1)*bal_M() = -bnd and snap_val < bnd
                // snapshot_payee bounds: sp_raw >= snap_raw and sp_raw <= snap_raw + amount
                assert(sp_raw as int >= snap_val);
                assert(sp_raw as int <= snap_val + amount as int);
                // old_val <= sp_raw (from payee invariant) and old_val >= sp_raw - amount (from reverse bound)
                assert(old_val as int <= sp_raw as int);
                assert(old_val as int <= bnd + i32::MAX as int);
                assert(old_val as int >= sp_raw as int - amount as int);
                assert(old_val as int >= -bnd - i32::MAX as int);
            }
            assert(old_val as int - amt as int >= i64::MIN as int) by(nonlinear_arith)
                requires old_val >= -bnd - i32::MAX as int,
                         amt >= 0i64, amt <= i32::MAX as i64,
                         bnd >= 0, bnd <= i32::MAX as int * (i32::MAX as int + 1);
            assert(old_val as int - amt as int <= i64::MAX as int) by(nonlinear_arith)
                requires old_val <= bnd + i32::MAX as int, amt >= 0i64;

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

        // Re-establish growing bound: bal_bounded(bal[k], b+2).
        proof {
            proof::bound_elem_from_nonneg_increase(balances@, snapshot_payee, amount as int);
            assert forall|k: int| 0 <= k < balances.len()
                implies bal_bounded(#[trigger] balances@[k], b as int + 2)
            by {
                assert(balances@[k] <= snapshot_payee[k]);
                assert(snapshot_payee[k] as int <= snapshot[k] as int + amount as int);
                assert(bal_bounded(snapshot[k], b as int + 1));
                assert(snapshot_payee[k] as int <= balances@[k] as int + amount as int);
                assert(snapshot_payee[k] as int >= snapshot[k] as int);
            }
        }

        b = b + 1;
    }

    // Final: bal_bounded(bal[k], b+1) where b+1 <= i32::MAX+1.
    // bal_M() * (i32::MAX+1) = 2^31 * 2^31 = 2^62 < i64::MAX.
    proof {
        assert forall|k: int| 0 <= k < balances.len()
            implies (#[trigger] balances@[k]) > i64::MIN && balances@[k] < i64::MAX
        by {
            assert(bal_bounded(balances@[k], b as int + 1));
            // bal_M() * (b+1) <= bal_M() * (i32::MAX+1) = 2^62 < i64::MAX.
            assert(spec::bal_M() * (b as int + 1) <= spec::bal_M() * (i32::MAX as int + 1)) by(nonlinear_arith)
                requires b as int + 1 <= i32::MAX as int + 1;
        }
    }

    balances
}

/// Find the index of a user_id in the users list.
/// Requires: the user exists (guaranteed by ledger_invariant + bill_well_formed).
pub fn find_user_index(users: &Vec<User>, user_id: u128) -> (idx: usize)
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
