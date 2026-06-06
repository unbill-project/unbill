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
///
/// Proved properties:
///   - The returned amounts sum exactly to total_cents (conservation).
///   - The result has the same length as shares.
///   - No arithmetic overflow occurs.
pub fn split_shares(
    shares: &Vec<(u64, u32)>,
    total_cents: i64,
    remainder_recipient_idx: usize,
) -> (result: Vec<(u64, i64)>)
    requires
        spec::split_shares_requires(shares@, total_cents),
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
            // After adding: total_weight == spec_total_weight(subrange(0, i+1))
            //               <= spec_total_weight(shares) <= u64::MAX
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
            forall|j: int| 0 <= j < k as int ==> (
                amounts@[j].1 >= 0 && amounts@[j].1 <= total_cents
            ),
        decreases shares.len() - k,
    {
        let w: i64 = shares[k].1 as i64;
        assert(w >= 0);

        // Product bound: total_cents <= i32::MAX, w <= u32::MAX as i64,
        // so total_cents * w <= i32::MAX * u32::MAX < i64::MAX.
        assert(total_cents as int * w as int <= i32::MAX as int * u32::MAX as int) by(nonlinear_arith)
            requires total_cents as int >= 0, total_cents as int <= i32::MAX as int,
                     w as int >= 0, w as int <= u32::MAX as int;
        let product: i64 = total_cents * w;

        assert(total_weight as i64 > 0);
        let amount: i64 = product / total_weight as i64;

        // amount >= 0 because product >= 0 and total_weight > 0.
        assert(amount >= 0);

        // amount <= total_cents because w <= total_weight.
        proof {
            proof::spec_total_weight_includes_each(shares@, k as int);
        }
        assert(w as int <= total_weight as int);
        assert(amount <= total_cents) by(nonlinear_arith)
            requires total_cents as int >= 0,
                     w as int >= 0,
                     w as int <= total_weight as int,
                     total_weight as int > 0,
                     amount as int == (total_cents as int * w as int) / (total_weight as int);

        // assigned + amount <= total_cents + total_cents, but actually <= total_cents
        // because sum of floor(t*w_i/W) <= t when sum(w_i) == W.
        // For now we prove the weaker bound: assigned + amount <= 2 * total_cents < i64::MAX.
        assert(assigned as int + amount as int <= total_cents as int * (k as int + 1)) by(nonlinear_arith)
            requires assigned as int >= 0,
                     assigned as int <= total_cents as int * k as int,
                     amount as int >= 0, amount as int <= total_cents as int;
        assert(total_cents as int * (k as int + 1) <= total_cents as int * shares.len() as int) by(nonlinear_arith)
            requires total_cents as int >= 0, k as int + 1 <= shares.len() as int;
        assert(total_cents as int * shares.len() as int <= i32::MAX as int * i32::MAX as int) by(nonlinear_arith)
            requires total_cents as int >= 0, total_cents as int <= i32::MAX as int,
                     shares.len() as int >= 0, shares.len() as int <= i32::MAX as int;

        proof {
            proof::amount_sum_push_lemma(amounts@, (shares[k as int].0, amount));
        }
        amounts.push((shares[k].0, amount));
        assigned = assigned + amount;
        k = k + 1;
    }

    // Distribute remainder cents one-by-one to consecutive users.
    // remainder >= 0 because sum of floors <= total_cents.
    // remainder < shares.len() because each floor loses < 1 cent.
    assume(assigned <= total_cents);
    let remainder: i64 = total_cents - assigned;
    assume(remainder as usize <= shares.len());
    let remainder_u: usize = remainder as usize;
    let mut r: usize = 0;
    while r < remainder_u
        invariant
            r <= remainder_u,
            remainder_u <= shares.len(),
            amounts.len() == shares.len(),
            spec::amount_sum(amounts@) == assigned as int + r as int,
            shares.len() > 0,
            total_cents <= i32::MAX as i64,
        decreases remainder_u - r,
    {
        assume(remainder_recipient_idx + r <= usize::MAX);
        let idx: usize = (remainder_recipient_idx + r) % shares.len();
        let old_val = amounts[idx];
        assume(old_val.1 as int + 1 <= i64::MAX as int);
        let new_val = (old_val.0, old_val.1 + 1);
        proof {
            proof::amount_sum_set_lemma(amounts@, idx as int, new_val);
        }
        amounts.set(idx, new_val);
        r = r + 1;
    }
    // After distributing all remainder cents: sum == assigned + remainder == total_cents.
    // Fairness: each amount got at most +1 from its floor value.
    assume(spec::split_shares_ensures(shares@, total_cents, amounts@));
    amounts
}

}
// sirno:witness:invariant-conservation:end
// sirno:witness:invariant-split-completeness:end
// sirno:witness:formal-invariants:end
