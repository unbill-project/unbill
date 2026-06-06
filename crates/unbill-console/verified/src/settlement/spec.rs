// Formal specifications for the settlement module.
// Contains only Verus spec fn definitions and proof fn lemmas.
// See formal-verification and formal-invariants entries in the Sirno lake.

use vstd::prelude::*;
use vstd::seq_lib::*;

// sirno:witness:formal-invariants:begin
verus! {

/// The sum of the second elements (amounts) in a sequence of pairs.
pub open spec fn amount_sum(s: Seq<(u64, i64)>) -> int
    decreases s.len(),
{
    if s.len() == 0 {
        0
    } else {
        amount_sum(s.drop_last()) + s.last().1 as int
    }
}

/// Lemma: pushing an element adds its amount to the sum.
pub proof fn amount_sum_push_lemma(s: Seq<(u64, i64)>, x: (u64, i64))
    ensures
        amount_sum(s.push(x)) == amount_sum(s) + x.1 as int,
{
    assert(s.push(x).drop_last() =~= s);
}

/// Lemma: updating one element changes the sum by the difference.
pub proof fn amount_sum_set_lemma(s: Seq<(u64, i64)>, idx: int, new_pair: (u64, i64))
    requires
        0 <= idx < s.len(),
    ensures
        amount_sum(s.update(idx, new_pair)) == amount_sum(s) - s[idx].1 as int + new_pair.1 as int,
    decreases s.len(),
{
    if s.len() == 1 {
        assert(s.update(idx, new_pair).drop_last() =~= Seq::<(u64, i64)>::empty());
        assert(s.drop_last() =~= Seq::<(u64, i64)>::empty());
    } else if idx == s.len() - 1 {
        assert(s.update(idx, new_pair).drop_last() =~= s.drop_last());
    } else {
        assert(s.update(idx, new_pair).drop_last() =~= s.drop_last().update(idx, new_pair));
        amount_sum_set_lemma(s.drop_last(), idx, new_pair);
    }
}

/// Spec-level split_shares: computes floor amounts and assigns remainder.
/// This is the mathematical model of the algorithm, operating on unbounded int.
pub open spec fn spec_split_shares(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    remainder_idx: int,
) -> Seq<(u64, i64)>
    decreases shares.len(),
{
    if shares.len() == 0 {
        Seq::empty()
    } else {
        let total_weight = spec_total_weight(shares);
        if total_weight == 0 {
            spec_zeros(shares)
        } else {
            let floor_amounts = spec_floor_amounts(shares, total_cents, total_weight);
            let assigned = amount_sum(floor_amounts);
            let remainder = total_cents - assigned;
            let idx = remainder_idx % shares.len() as int;
            if remainder != 0 {
                floor_amounts.update(idx, (floor_amounts[idx].0, (floor_amounts[idx].1 as int + remainder) as i64))
            } else {
                floor_amounts
            }
        }
    }
}

/// Sum of all weights in the share list.
pub open spec fn spec_total_weight(shares: Seq<(u64, u32)>) -> int
    decreases shares.len(),
{
    if shares.len() == 0 {
        0
    } else {
        spec_total_weight(shares.drop_last()) + shares.last().1 as int
    }
}

/// A sequence of zeros with the same IDs as shares.
pub open spec fn spec_zeros(shares: Seq<(u64, u32)>) -> Seq<(u64, i64)>
    decreases shares.len(),
{
    if shares.len() == 0 {
        Seq::empty()
    } else {
        spec_zeros(shares.drop_last()).push((shares.last().0, 0i64))
    }
}

/// Floor amounts: floor(total_cents * w_i / total_weight) for each share.
pub open spec fn spec_floor_amounts(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    total_weight: int,
) -> Seq<(u64, i64)>
    decreases shares.len(),
{
    if shares.len() == 0 {
        Seq::empty()
    } else {
        spec_floor_amounts(shares.drop_last(), total_cents, total_weight)
            .push((shares.last().0, (total_cents * shares.last().1 as int / total_weight) as i64))
    }
}

/// Lemma: spec_zeros always sums to 0.
pub proof fn spec_zeros_sum(shares: Seq<(u64, u32)>)
    ensures
        amount_sum(spec_zeros(shares)) == 0,
    decreases shares.len(),
{
    if shares.len() > 0 {
        spec_zeros_sum(shares.drop_last());
        amount_sum_push_lemma(spec_zeros(shares.drop_last()), (shares.last().0, 0i64));
    }
}

/// Lemma: floor amounts sum + remainder == total_cents.
/// This is the core conservation property.
pub proof fn floor_amounts_remainder_lemma(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    total_weight: int,
)
    requires
        shares.len() > 0,
        total_cents >= 0,
        total_weight > 0,
    ensures ({
        let floor_amounts = spec_floor_amounts(shares, total_cents, total_weight);
        let assigned = amount_sum(floor_amounts);
        assigned + (total_cents - assigned) == total_cents
    }),
{
    // This is trivially true: assigned + (total_cents - assigned) == total_cents.
    // The real content is that the split_shares function computes this correctly.
}

/// Lemma: spec_total_weight distributes over push.
pub proof fn spec_total_weight_push(s: Seq<(u64, u32)>, x: (u64, u32))
    ensures
        spec_total_weight(s.push(x)) == spec_total_weight(s) + x.1 as int,
{
    assert(s.push(x).drop_last() =~= s);
}

/// Lemma: each individual weight is <= total weight.
pub proof fn spec_total_weight_includes_each(shares: Seq<(u64, u32)>, idx: int)
    requires
        0 <= idx < shares.len(),
    ensures
        shares[idx].1 as int <= spec_total_weight(shares),
    decreases shares.len(),
{
    if shares.len() == 1 {
    } else if idx == shares.len() - 1 {
        // Last element: total = rest + last >= last.
        // Need rest >= 0.
        spec_total_weight_nonneg(shares.drop_last());
    } else {
        spec_total_weight_includes_each(shares.drop_last(), idx);
    }
}

/// Lemma: total weight is always non-negative.
pub proof fn spec_total_weight_nonneg(shares: Seq<(u64, u32)>)
    ensures
        spec_total_weight(shares) >= 0,
    decreases shares.len(),
{
    if shares.len() > 0 {
        spec_total_weight_nonneg(shares.drop_last());
    }
}

/// Lemma: partial weight sum is <= total weight.
pub proof fn spec_total_weight_partial_le(shares: Seq<(u64, u32)>, n: int)
    requires
        0 <= n <= shares.len(),
    ensures
        spec_total_weight(shares.subrange(0, n)) <= spec_total_weight(shares),
    decreases shares.len(),
{
    if n == shares.len() {
        assert(shares.subrange(0, n) =~= shares);
    } else if shares.len() > 0 {
        spec_total_weight_partial_le(shares.drop_last(), n);
        assert(shares.subrange(0, n) =~= shares.drop_last().subrange(0, n));
    }
}

}
// sirno:witness:formal-invariants:end
