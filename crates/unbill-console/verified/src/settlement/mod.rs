// Verified settlement math: split shares and minimum-cash-flow reduction.
// This is the production code — unbill-console calls these functions at runtime.

use unbill_model_verified::ledger::exec::*;
use unbill_model_verified::ledger::proof::*;
use unbill_model_verified::ledger::spec::*;
use vstd::prelude::*;
use vstd::slice::SliceAdditionalExecFns;

pub mod effective;
pub mod exec;
pub mod proof;
pub mod spec;

// sirno:witness:formal-invariants:begin
// sirno:witness:invariant-split-completeness:begin
// sirno:witness:invariant-conservation:begin
verus! {

/// Compute the per-share cent amounts from a share list and a total.
/// Returns a Vec<i64> of amounts indexed by position in shares.
pub fn split_shares(
    shares: &Vec<Share>,
    total_cents: i64,
    remainder_recipient_idx: usize,
) -> (result: Vec<i64>)
    requires
        split_shares_requires(shares_to_specs(shares@), total_cents),
        // Bounded index to avoid usize overflow in remainder loop.
        remainder_recipient_idx <= usize::MAX - shares.len(),
    ensures
        spec::split_shares_ensures(shares_to_specs(shares@), total_cents, result@),
{
    let ghost spec_shares: Seq<ShareSpec> = shares_to_specs(shares@);

    // Sum weights.
    let mut tw: u64 = 0;
    let mut i: usize = 0;
    while i < shares.len()
        invariant
            i <= shares.len(),
            spec_shares == shares_to_specs(shares@),
            tw as int == total_weight(spec_shares.subrange(0, i as int)),
            tw as int <= total_weight(spec_shares),
            total_weight(spec_shares) <= u64::MAX as int,
        decreases shares.len() - i,
    {
        proof {
            // spec_shares[i].weight == shares@[i].weight by Seq::new axiom
            total_weight_push(spec_shares.subrange(0, i as int), spec_shares[i as int]);
            assert(spec_shares.subrange(0, i as int).push(spec_shares[i as int])
                =~= spec_shares.subrange(0, (i + 1) as int));
            total_weight_partial_le(spec_shares, (i + 1) as int);
        }
        tw = tw + shares[i].weight as u64;
        i = i + 1;
    }
    proof {
        assert(spec_shares.subrange(0, spec_shares.len() as int) =~= spec_shares);
    }
    assert(tw > 0);
    assert(tw as int <= i64::MAX as int);

    // Compute floor amounts and track the running sum.
    let mut amounts: Vec<i64> = Vec::new();
    let mut assigned: i64 = 0;
    let mut k: usize = 0;
    while k < shares.len()
        invariant
            k <= shares.len(),
            amounts.len() == k,
            spec_shares == shares_to_specs(shares@),
            spec::seq_sum(amounts@) == assigned as int,
            assigned >= 0,
            assigned as int <= total_cents as int * k as int,
            tw > 0,
            tw as int <= i64::MAX as int,
            total_cents >= 0,
            total_cents <= i32::MAX as i64,
            shares.len() <= i32::MAX as usize,
            tw as int == total_weight(spec_shares),
            // Track that each amount equals its floor and is bounded.
            forall|j: int| 0 <= j < k as int ==> (
                amounts@[j] >= 0
                && amounts@[j] <= total_cents
                && amounts@[j] as int == (total_cents as int * spec_shares[j].weight as int) / (tw as int)
            ),
        decreases shares.len() - k,
    {
        let w: i64 = shares[k].weight as i64;
        assert(w >= 0);
        assert(total_cents as int * w as int <= i32::MAX as int * u32::MAX as int) by(nonlinear_arith)
            requires total_cents as int >= 0, total_cents as int <= i32::MAX as int,
                     w as int >= 0, w as int <= u32::MAX as int;
        let product: i64 = total_cents * w;
        assert(tw as i64 > 0);
        let amount: i64 = product / tw as i64;
        assert(amount >= 0);
        proof { total_weight_includes_each(spec_shares, k as int); }
        assert(w as int <= tw as int);
        assert(amount <= total_cents) by(nonlinear_arith)
            requires total_cents as int >= 0, w as int >= 0, w as int <= tw as int,
                     tw as int > 0,
                     amount as int == (total_cents as int * w as int) / (tw as int);
        assert(assigned as int + amount as int <= total_cents as int * (k as int + 1)) by(nonlinear_arith)
            requires assigned as int >= 0, assigned as int <= total_cents as int * k as int,
                     amount as int >= 0, amount as int <= total_cents as int;
        assert(total_cents as int * (k as int + 1) <= total_cents as int * shares.len() as int) by(nonlinear_arith)
            requires total_cents as int >= 0, k as int + 1 <= shares.len() as int;
        assert(total_cents as int * shares.len() as int <= i32::MAX as int * i32::MAX as int) by(nonlinear_arith)
            requires total_cents as int >= 0, total_cents as int <= i32::MAX as int,
                     shares.len() as int >= 0, shares.len() as int <= i32::MAX as int;
        proof { proof::seq_sum_push(amounts@, amount); }
        amounts.push(amount);
        assigned = assigned + amount;
        k = k + 1;
    }

    // Use proven lemmas: assigned == floor_sum, so assigned <= total_cents
    // and remainder < shares.len().
    proof {
        proof::floor_sum_le_total(spec_shares, total_cents as int, tw as int);
        proof::floor_sum_remainder_lt_n(spec_shares, total_cents as int, tw as int);
        // Connect assigned to floor_sum: we tracked that each amount[j] == floor(t*w_j/W).
        proof::floor_sum_eq_seq_sum(spec_shares, amounts@, total_cents as int, tw as int);
    }
    assert(assigned <= total_cents);
    let remainder: i64 = total_cents - assigned;
    assert(remainder >= 0);
    assert((remainder as usize) < shares.len());
    let remainder_u: usize = remainder as usize;

    // Distribute remainder cents one-by-one to consecutive users.
    // Ghost: save the floor amounts before distributing remainder.
    let ghost floor_amounts = amounts@;

    let mut r: usize = 0;
    while r < remainder_u
        invariant
            r <= remainder_u,
            remainder_u < shares.len(),
            amounts.len() == shares.len(),
            spec::seq_sum(amounts@) == assigned as int + r as int,
            shares.len() > 0,
            total_cents >= 0,
            total_cents <= i32::MAX as i64,
            assigned >= 0,
            remainder_recipient_idx + shares.len() <= usize::MAX,
            tw as int == total_weight(spec_shares),
            tw > 0,
            // Visited indices got +1, unvisited indices unchanged.
            forall|j: int| 0 <= j < amounts@.len() ==> (
                amounts@[j] == floor_amounts[j]
                || amounts@[j] == floor_amounts[j] + 1
            ),
            // Unvisited indices still at floor value.
            forall|j: int| 0 <= j < amounts@.len() ==> (
                (forall|r2: int| 0 <= r2 < r as int ==>
                    #[trigger] ((remainder_recipient_idx as int + r2) % (shares.len() as int)) != j
                ) ==> amounts@[j] == floor_amounts[j]
            ),
            // Floor amounts are bounded (len matches shares).
            floor_amounts.len() == shares.len(),
            forall|j: int| 0 <= j < shares.len() ==> (
                #[trigger] floor_amounts[j] >= 0 && floor_amounts[j] <= total_cents
            ),
        decreases remainder_u - r,
    {
        assert(remainder_recipient_idx + r < remainder_recipient_idx + shares.len());
        let idx: usize = (remainder_recipient_idx + r) % shares.len();
        let old_val = amounts[idx];

        // Prove old_val is at floor (not yet incremented) via distinct indices.
        proof {
            assert forall|r2: int| 0 <= r2 < r as int
                implies #[trigger] ((remainder_recipient_idx as int + r2) % (shares.len() as int))
                    != idx as int
            by {
                proof::mod_distinct(
                    remainder_recipient_idx as int, r2, r as int, shares.len() as int,
                );
            }
            // By the "unvisited" invariant: amounts[idx] == floor_amounts[idx].
        }
        assert(old_val == floor_amounts[idx as int]);
        assert(old_val >= 0);
        proof {
            assert(floor_amounts[idx as int] <= total_cents);
        }

        let new_val = old_val + 1;
        proof { proof::seq_sum_update(amounts@, idx as int, new_val); }
        amounts.set(idx, new_val);
        r = r + 1;
    }

    // After loop: sum == assigned + remainder == total_cents.
    assert(spec::seq_sum(amounts@) == total_cents as int);
    assert(amounts@.len() == shares.len());
    // Fairness: each amount is floor or floor+1.
    assert(forall|j: int| 0 <= j < amounts@.len() ==> (
        amounts@[j] == floor_amounts[j]
        || amounts@[j] == floor_amounts[j] + 1
    ));
    // Connect floor_amounts to floor_amount spec.
    proof {
        assert forall|j: int| 0 <= j < amounts@.len()
            implies #[trigger] (amounts@[j] as int) >= spec::floor_amount(spec_shares, total_cents as int, j)
                && amounts@[j] as int <= spec::floor_amount(spec_shares, total_cents as int, j) + 1
        by {
            assert(floor_amounts[j] as int
                == (total_cents as int * spec_shares[j].weight as int) / (tw as int));
            assert(spec::floor_amount(spec_shares, total_cents as int, j)
                == (total_cents as int * spec_shares[j].weight as int) / total_weight(spec_shares));
        }
    }
    amounts
}

/// Full settlement pipeline: filter effective bills → split → accumulate → settle.
/// Takes a LedgerState and remainder indices, returns settlement transactions.
pub fn compute_settlement(
    ledger: &LedgerState,
    remainder_indices: &Vec<usize>,
) -> (transactions: Vec<exec::Transaction>)
    requires
        ledger_invariant(ledger@),
        remainder_indices.len() == ledger.bills.len(),
        // All prev references valid at exec level.
        forall|i: int| #![trigger ledger.bills@[i]]
            0 <= i < ledger.bills.len() ==>
            forall|k: int| #![trigger ledger.bills@[i].prev@[k]]
                0 <= k < ledger.bills@[i].prev.len() ==>
                exists|j: int| 0 <= j < ledger.bills.len() && (#[trigger] ledger.bills@[j]).id == ledger.bills@[i].prev@[k],
        // Exec-level user existence for all shares.
        forall|i: int, s: int| #![trigger ledger.bills@[i].payers@[s]]
            0 <= i < ledger.bills.len() && 0 <= s < ledger.bills@[i].payers.len() ==>
            exists|k: int| 0 <= k < ledger.users.len()
                && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[i].payers@[s].user_id,
        forall|i: int, s: int| #![trigger ledger.bills@[i].payees@[s]]
            0 <= i < ledger.bills.len() && 0 <= s < ledger.bills@[i].payees.len() ==>
            exists|k: int| 0 <= k < ledger.users.len()
                && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[i].payees@[s].user_id,
        // Overflow bounds.
        ledger.bills.len() <= i32::MAX as usize,
        // split_shares preconditions at exec level.
        forall|i: int| #![trigger ledger.bills@[i]]
            0 <= i < ledger.bills.len() ==>
            split_shares_requires(shares_to_specs(ledger.bills@[i].payers@), ledger.bills@[i].amount_cents)
            && split_shares_requires(shares_to_specs(ledger.bills@[i].payees@), ledger.bills@[i].amount_cents),
        // Remainder indices bounded.
        forall|i: int| 0 <= i < remainder_indices.len() ==>
            (#[trigger] remainder_indices@[i]) <= usize::MAX - (i32::MAX as usize),
{
    // Step 1: Filter effective bills.
    let effective_indices = effective::filter_effective_indices(&ledger.bills);

    // Step 2+3: Accumulate balances from effective bills.
    let mut balances: Vec<i64> = Vec::new();
    let mut user_ids: Vec<u128> = Vec::new();
    let mut u: usize = 0;
    while u < ledger.users.len()
        invariant
            u <= ledger.users.len(),
            balances.len() == u,
            user_ids.len() == u,
            spec::seq_sum(balances@) == 0,
        decreases ledger.users.len() - u,
    {
        proof { proof::seq_sum_push(balances@, 0i64); }
        balances.push(0i64);
        user_ids.push(ledger.users[u].user_id);
        u = u + 1;
    }

    // Step 2: Process each effective bill — split and accumulate.
    let mut ei: usize = 0;
    while ei < effective_indices.len()
        invariant
            ei <= effective_indices.len(),
            balances.len() == ledger.users.len(),
            user_ids.len() == ledger.users.len(),
            spec::seq_sum(balances@) == 0,
            // Effective indices are valid bill indices.
            forall|k: int| 0 <= k < effective_indices.len() ==>
                (#[trigger] effective_indices@[k]) < ledger.bills.len(),
            effective_indices.len() <= ledger.bills.len(),
            // Carry preconditions through.
            ledger_invariant(ledger@),
            ledger.bills.len() <= i32::MAX as usize,
            forall|i: int| #![trigger ledger.bills@[i]]
                0 <= i < ledger.bills.len() ==>
                forall|k: int| #![trigger ledger.bills@[i].prev@[k]]
                    0 <= k < ledger.bills@[i].prev.len() ==>
                    exists|j: int| 0 <= j < ledger.bills.len() && (#[trigger] ledger.bills@[j]).id == ledger.bills@[i].prev@[k],
            forall|i: int, s: int| #![trigger ledger.bills@[i].payers@[s]]
                0 <= i < ledger.bills.len() && 0 <= s < ledger.bills@[i].payers.len() ==>
                exists|k: int| 0 <= k < ledger.users.len()
                    && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[i].payers@[s].user_id,
            forall|i: int, s: int| #![trigger ledger.bills@[i].payees@[s]]
                0 <= i < ledger.bills.len() && 0 <= s < ledger.bills@[i].payees.len() ==>
                exists|k: int| 0 <= k < ledger.users.len()
                    && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[i].payees@[s].user_id,
            forall|i: int| #![trigger ledger.bills@[i]]
                0 <= i < ledger.bills.len() ==>
                split_shares_requires(shares_to_specs(ledger.bills@[i].payers@), ledger.bills@[i].amount_cents)
                && split_shares_requires(shares_to_specs(ledger.bills@[i].payees@), ledger.bills@[i].amount_cents),
            forall|i: int| 0 <= i < remainder_indices.len() ==>
                (#[trigger] remainder_indices@[i]) <= usize::MAX - (i32::MAX as usize),
            remainder_indices.len() == ledger.bills.len(),
        decreases effective_indices.len() - ei,
    {
        let bill_idx = effective_indices[ei];
        let bill = &ledger.bills[bill_idx];
        let rem_idx = remainder_indices[bill_idx];

        // Bridge: connect ledger_invariant (spec) to exec-level split_shares precondition.
        proof {
            // ledger@.bills[i] == ledger.bills@[i]@ — connects View to direct access.
            unbill_model_verified::ledger::exec::ledger_bill_at(*ledger, bill_idx as int);
            // Now: bill_splittable(ledger@.bills[bill_idx]) == bill_splittable(ledger.bills@[bill_idx]@)
            // bill_splittable_bridge needs bill_splittable(bill@):
            unbill_model_verified::ledger::exec::bill_splittable_bridge(ledger.bills@[bill_idx as int]);
        }

        // Split payers and payees.
        let payer_amounts = split_shares(&bill.payers, bill.amount_cents, rem_idx);
        let payee_amounts = split_shares(&bill.payees, bill.amount_cents, rem_idx);

        // Accumulate payer credits.
        let payer_len = bill.payers.len();
        let mut i: usize = 0;
        while i < payer_amounts.len()
            invariant
                i <= payer_amounts.len(),
                payer_amounts.len() == payer_len,
                payer_len == ledger.bills@[bill_idx as int].payers.len(),
                balances.len() == ledger.users.len(),
                spec::seq_sum(balances@) == spec::seq_sum(payer_amounts@.subrange(0, i as int)),
                bill_idx < ledger.bills.len(),
                forall|s: int| #![trigger ledger.bills@[bill_idx as int].payers@[s]]
                    0 <= s < ledger.bills@[bill_idx as int].payers.len() ==>
                    exists|k: int| 0 <= k < ledger.users.len()
                        && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[bill_idx as int].payers@[s].user_id,
            decreases payer_amounts.len() - i,
        {
            let uid: u128 = ledger.bills[bill_idx].payers[i].user_id;
            let user_idx = exec::find_user_index(&ledger.users, uid);
            let old_val = balances[user_idx];
            let amt = payer_amounts[i];
            assume(old_val as int + amt as int <= i64::MAX as int);
            assume(old_val as int + amt as int >= i64::MIN as int);
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

        // Accumulate payee debits.
        let payee_len = bill.payees.len();
        let mut j: usize = 0;
        while j < payee_amounts.len()
            invariant
                j <= payee_amounts.len(),
                payee_amounts.len() == payee_len,
                payee_len == ledger.bills@[bill_idx as int].payees.len(),
                balances.len() == ledger.users.len(),
                spec::seq_sum(balances@)
                    == spec::seq_sum(payer_amounts@) - spec::seq_sum(payee_amounts@.subrange(0, j as int)),
                bill_idx < ledger.bills.len(),
                forall|s: int| #![trigger ledger.bills@[bill_idx as int].payees@[s]]
                    0 <= s < ledger.bills@[bill_idx as int].payees.len() ==>
                    exists|k: int| 0 <= k < ledger.users.len()
                        && (#[trigger] ledger.users@[k]).user_id == ledger.bills@[bill_idx as int].payees@[s].user_id,
            decreases payee_amounts.len() - j,
        {
            let uid: u128 = ledger.bills[bill_idx].payees[j].user_id;
            let user_idx = exec::find_user_index(&ledger.users, uid);
            let old_val = balances[user_idx];
            let amt = payee_amounts[j];
            assume(old_val as int - amt as int >= i64::MIN as int);
            assume(old_val as int - amt as int <= i64::MAX as int);
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
            // seq_sum(balances@) == payer_total - payee_total == amount_cents - amount_cents == 0.
        }

        ei = ei + 1;
    }

    // Step 3: Greedy matching.
    assume(spec::settle_requires(balances@));
    let transactions = exec::compute_from_balances(&user_ids, &balances);
    transactions
}

}
// sirno:witness:invariant-conservation:end
// sirno:witness:invariant-split-completeness:end
// sirno:witness:formal-invariants:end
