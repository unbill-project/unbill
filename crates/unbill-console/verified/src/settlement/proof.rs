// Proof utilities for the settlement module.
// Contains proof fn lemmas and helper spec fn definitions used to guide proofs.
// The interface specs they prove properties about live in spec.rs.

use super::spec::*;
use vstd::prelude::*;
use vstd::seq_lib::*;

// sirno:witness:formal-invariants:begin
verus! {

// ---------------------------------------------------------------------------
// amount_sum lemmas
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// spec_total_weight lemmas
// ---------------------------------------------------------------------------

/// Lemma: spec_total_weight distributes over push.
pub proof fn spec_total_weight_push(s: Seq<(u64, u32)>, x: (u64, u32))
    ensures
        spec_total_weight(s.push(x)) == spec_total_weight(s) + x.1 as int,
{
    assert(s.push(x).drop_last() =~= s);
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
        spec_total_weight_nonneg(shares.drop_last());
    } else {
        spec_total_weight_includes_each(shares.drop_last(), idx);
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
