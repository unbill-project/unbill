// Verified settlement math: split shares and minimum-cash-flow reduction.
// This is the production code — unbill-console calls these functions at runtime.

use vstd::prelude::*;
use vstd::slice::SliceAdditionalExecFns;

pub mod proof;
pub mod spec;

// sirno:witness:formal-invariants:begin
// sirno:witness:invariant-split-completeness:begin
// sirno:witness:invariant-conservation:begin
verus! {

/// Compute the per-user cent amounts from a share list and a total.
pub fn split_shares(
    shares: &Vec<(u64, u32)>,
    total_cents: i64,
    remainder_recipient_idx: usize,
) -> (result: Vec<(u64, i64)>)
    requires
        spec::split_shares_requires(shares@, total_cents),
        // Bounded index to avoid usize overflow in remainder loop.
        remainder_recipient_idx <= usize::MAX - shares.len(),
    ensures
        spec::split_shares_ensures(shares@, total_cents, result@),
{
    // Sum weights.
    let mut total_weight: u64 = 0;
    let mut i: usize = 0;
    while i < shares.len()
        invariant
            i <= shares.len(),
            total_weight as int == spec::spec_total_weight(shares@.subrange(0, i as int)),
            total_weight as int <= spec::spec_total_weight(shares@),
            spec::spec_total_weight(shares@) <= u64::MAX as int,
        decreases shares.len() - i,
    {
        proof {
            proof::spec_total_weight_push(shares@.subrange(0, i as int), shares[i as int]);
            assert(shares@.subrange(0, i as int).push(shares[i as int])
                =~= shares@.subrange(0, (i + 1) as int));
            proof::spec_total_weight_partial_le(shares@, (i + 1) as int);
        }
        total_weight = total_weight + shares[i].1 as u64;
        i = i + 1;
    }
    proof {
        assert(shares@.subrange(0, shares@.len() as int) =~= shares@);
    }
    assert(total_weight > 0);
    assert(total_weight as int <= i64::MAX as int);

    // Compute floor amounts and track the running sum.
    let mut amounts: Vec<(u64, i64)> = Vec::new();
    let mut assigned: i64 = 0;
    let mut k: usize = 0;
    while k < shares.len()
        invariant
            k <= shares.len(),
            amounts.len() == k,
            spec::amount_sum(amounts@) == assigned as int,
            assigned >= 0,
            assigned as int <= total_cents as int * k as int,
            total_weight > 0,
            total_weight as int <= i64::MAX as int,
            total_cents >= 0,
            total_cents <= i32::MAX as i64,
            shares.len() <= i32::MAX as usize,
            total_weight as int == spec::spec_total_weight(shares@),
            // Track that each amount equals its floor and is bounded.
            forall|j: int| 0 <= j < k as int ==> (
                amounts@[j].1 >= 0
                && amounts@[j].1 <= total_cents
                && amounts@[j].1 as int == (total_cents as int * shares@[j].1 as int) / (total_weight as int)
            ),
        decreases shares.len() - k,
    {
        let w: i64 = shares[k].1 as i64;
        assert(w >= 0);
        assert(total_cents as int * w as int <= i32::MAX as int * u32::MAX as int) by(nonlinear_arith)
            requires total_cents as int >= 0, total_cents as int <= i32::MAX as int,
                     w as int >= 0, w as int <= u32::MAX as int;
        let product: i64 = total_cents * w;
        assert(total_weight as i64 > 0);
        let amount: i64 = product / total_weight as i64;
        assert(amount >= 0);
        proof { proof::spec_total_weight_includes_each(shares@, k as int); }
        assert(w as int <= total_weight as int);
        assert(amount <= total_cents) by(nonlinear_arith)
            requires total_cents as int >= 0, w as int >= 0, w as int <= total_weight as int,
                     total_weight as int > 0,
                     amount as int == (total_cents as int * w as int) / (total_weight as int);
        assert(assigned as int + amount as int <= total_cents as int * (k as int + 1)) by(nonlinear_arith)
            requires assigned as int >= 0, assigned as int <= total_cents as int * k as int,
                     amount as int >= 0, amount as int <= total_cents as int;
        assert(total_cents as int * (k as int + 1) <= total_cents as int * shares.len() as int) by(nonlinear_arith)
            requires total_cents as int >= 0, k as int + 1 <= shares.len() as int;
        assert(total_cents as int * shares.len() as int <= i32::MAX as int * i32::MAX as int) by(nonlinear_arith)
            requires total_cents as int >= 0, total_cents as int <= i32::MAX as int,
                     shares.len() as int >= 0, shares.len() as int <= i32::MAX as int;
        proof { proof::amount_sum_push_lemma(amounts@, (shares[k as int].0, amount)); }
        amounts.push((shares[k].0, amount));
        assigned = assigned + amount;
        k = k + 1;
    }

    // Use proven lemmas: assigned == floor_sum, so assigned <= total_cents
    // and remainder < shares.len().
    proof {
        proof::floor_sum_le_total(shares@, total_cents as int, total_weight as int);
        proof::floor_sum_remainder_lt_n(shares@, total_cents as int, total_weight as int);
        // Connect assigned to floor_sum: we tracked that each amount[j].1 == floor(t*w_j/W).
        // So amount_sum(amounts@) == floor_sum(shares@, t, W).
        // Proved via: assigned == amount_sum(amounts@) and each element matches.
        proof::floor_sum_eq_amount_sum(shares@, amounts@, total_cents as int, total_weight as int);
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
            spec::amount_sum(amounts@) == assigned as int + r as int,
            shares.len() > 0,
            total_cents >= 0,
            total_cents <= i32::MAX as i64,
            assigned >= 0,
            remainder_recipient_idx + shares.len() <= usize::MAX,
            total_weight as int == spec::spec_total_weight(shares@),
            total_weight > 0,
            // Visited indices got +1, unvisited indices unchanged.
            forall|j: int| 0 <= j < amounts@.len() ==> (
                amounts@[j].1 == floor_amounts[j].1
                || amounts@[j].1 == floor_amounts[j].1 + 1
            ),
            // Unvisited indices still at floor value.
            forall|j: int| 0 <= j < amounts@.len() ==> (
                (forall|r2: int| 0 <= r2 < r as int ==>
                    #[trigger] ((remainder_recipient_idx as int + r2) % (shares.len() as int)) != j
                ) ==> amounts@[j].1 == floor_amounts[j].1
            ),
            // Floor amounts are bounded (len matches shares).
            floor_amounts.len() == shares.len(),
            forall|j: int| 0 <= j < shares.len() ==> (
                #[trigger] floor_amounts[j].1 >= 0 && floor_amounts[j].1 <= total_cents
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
            // By the "unvisited" invariant: amounts[idx].1 == floor_amounts[idx].1.
        }
        assert(old_val.1 == floor_amounts[idx as int].1);
        assert(old_val.1 >= 0);
        proof {
            // floor_amounts invariant: forall j in [0, len) ==> floor_amounts[j].1 <= total_cents.
            // idx < shares.len() == floor_amounts length.
            assert(floor_amounts[idx as int].1 <= total_cents);
        }

        let new_val = (old_val.0, old_val.1 + 1);
        proof { proof::amount_sum_set_lemma(amounts@, idx as int, new_val); }
        amounts.set(idx, new_val);
        r = r + 1;
    }

    // After loop: sum == assigned + remainder == total_cents.
    assert(spec::amount_sum(amounts@) == total_cents as int);
    assert(amounts@.len() == shares.len());
    // Fairness: each amount is floor or floor+1.
    assert(forall|j: int| 0 <= j < amounts@.len() ==> (
        amounts@[j].1 == floor_amounts[j].1
        || amounts@[j].1 == floor_amounts[j].1 + 1
    ));
    // Connect floor_amounts to floor_amount spec.
    proof {
        assert forall|j: int| 0 <= j < amounts@.len()
            implies #[trigger] (amounts@[j].1 as int) >= spec::floor_amount(shares@, total_cents as int, j)
                && amounts@[j].1 as int <= spec::floor_amount(shares@, total_cents as int, j) + 1
        by {
            assert(floor_amounts[j].1 as int
                == (total_cents as int * shares@[j].1 as int) / (total_weight as int));
            assert(spec::floor_amount(shares@, total_cents as int, j)
                == (total_cents as int * shares@[j].1 as int) / spec::spec_total_weight(shares@));
        }
    }
    amounts
}

}
// sirno:witness:invariant-conservation:end
// sirno:witness:invariant-split-completeness:end
// sirno:witness:formal-invariants:end
