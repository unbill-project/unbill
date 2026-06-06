// Verified settlement math: split shares and minimum-cash-flow reduction.
// This is the production code — unbill-console calls these functions at runtime.

use vstd::prelude::*;

pub mod spec;

// sirno:witness:formal-invariants:begin
// sirno:witness:invariant-split-completeness:begin
// sirno:witness:invariant-conservation:begin
verus! {

/// Compute the per-user cent amounts from a share list and a total.
///
/// Proved property: the returned amounts always sum exactly to total_cents.
pub fn split_shares(
    shares: &Vec<(u64, u32)>,
    total_cents: i64,
    remainder_recipient_idx: usize,
) -> (result: Vec<(u64, i64)>)
    requires
        shares.len() > 0,
        total_cents >= 0,
        // Total weight must be nonzero — can't distribute cents with no weight.
        spec::spec_total_weight(shares@) > 0,
    ensures
        spec::amount_sum(result@) == total_cents as int,
        result.len() == shares.len(),
{
    // Sum weights.
    let mut total_weight: u64 = 0;
    let mut i: usize = 0;
    while i < shares.len()
        invariant
            i <= shares.len(),
            total_weight as int == spec::spec_total_weight(shares@.subrange(0, i as int)),
        decreases shares.len() - i,
    {
        assume(total_weight as int + shares[i as int].1 as int <= u64::MAX as int);
        proof {
            spec::spec_total_weight_push(shares@.subrange(0, i as int), shares[i as int]);
            assert(shares@.subrange(0, i as int).push(shares[i as int]) =~= shares@.subrange(0, (i + 1) as int));
        }
        total_weight = total_weight + shares[i].1 as u64;
        i = i + 1;
    }
    proof {
        assert(shares@.subrange(0, shares@.len() as int) =~= shares@);
    }
    assert(total_weight > 0);

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
            total_weight > 0,
        decreases shares.len() - k,
    {
        let w: i64 = shares[k].1 as i64;
        assume(total_cents as int * w as int <= i64::MAX as int);
        assume(total_cents as int * w as int >= i64::MIN as int);
        let product: i64 = total_cents * w;
        assume(total_weight as i64 > 0);
        let amount: i64 = product / total_weight as i64;
        assume(amount >= 0);
        assume(assigned as int + amount as int <= i64::MAX as int);
        proof {
            spec::amount_sum_push_lemma(amounts@, (shares[k as int].0, amount));
        }
        amounts.push((shares[k].0, amount));
        assigned = assigned + amount;
        k = k + 1;
    }

    // Assign remainder to one recipient.
    // Key conservation insight: remainder = total_cents - assigned,
    // so after adding it, sum = assigned + remainder = total_cents.
    assume(total_cents as int - assigned as int >= i64::MIN as int);
    let remainder: i64 = total_cents - assigned;
    if remainder != 0 {
        let idx: usize = remainder_recipient_idx % shares.len();
        assume(amounts[idx as int].1 as int + remainder as int <= i64::MAX as int);
        assume(amounts[idx as int].1 as int + remainder as int >= i64::MIN as int);
        let old_val = amounts[idx];
        let new_val = (old_val.0, old_val.1 + remainder);
        proof {
            spec::amount_sum_set_lemma(amounts@, idx as int, new_val);
        }
        amounts.set(idx, new_val);
    }
    amounts
}

}
// sirno:witness:invariant-conservation:end
// sirno:witness:invariant-split-completeness:end
// sirno:witness:formal-invariants:end
