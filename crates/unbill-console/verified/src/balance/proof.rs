// Proof utilities for the balance settlement module.

use super::spec::*;
use vstd::prelude::*;

verus! {

/// seq_sum distributes over push.
pub proof fn seq_sum_push(s: Seq<i64>, x: i64)
    ensures
        seq_sum(s.push(x)) == seq_sum(s) + x as int,
{
    assert(s.push(x).drop_last() =~= s);
}

/// transaction_sum distributes over push.
pub proof fn transaction_sum_push(s: Seq<TransactionSpec>, t: TransactionSpec)
    ensures
        transaction_sum(s.push(t)) == transaction_sum(s) + t.amount_cents,
{
    assert(s.push(t).drop_last() =~= s);
}

/// range_sum(s, 0, s.len()) == seq_sum(s).
pub proof fn range_sum_eq_seq_sum(s: Seq<i64>)
    ensures range_sum(s, 0, s.len() as int) == seq_sum(s),
    decreases s.len(),
{
    if s.len() > 0 {
        // seq_sum(s) = seq_sum(s.drop_last()) + s.last()
        // range_sum(s, 0, n) = s[0] + range_sum(s, 1, n)
        // Need to relate these two decompositions.
        // Strategy: show range_sum(s, 0, n) = range_sum(s, 0, n-1) + s[n-1].
        range_sum_split_last(s, 0, s.len() as int);
        // Now: range_sum(s, 0, n) == range_sum(s, 0, n-1) + s[n-1].
        // And: seq_sum(s) == seq_sum(s.drop_last()) + s.last() == seq_sum(s.drop_last()) + s[n-1].
        // IH on s.drop_last(): range_sum(s.drop_last(), 0, n-1) == seq_sum(s.drop_last()).
        range_sum_eq_seq_sum(s.drop_last());
        // Need: range_sum(s, 0, n-1) == range_sum(s.drop_last(), 0, n-1).
        range_sum_prefix_eq(s, s.drop_last(), 0, s.len() as int - 1);
    }
}

/// range_sum(s, from, to) == range_sum(s, from, to-1) + s[to-1].
proof fn range_sum_split_last(s: Seq<i64>, from: int, to: int)
    requires 0 <= from < to, to <= s.len(),
    ensures range_sum(s, from, to) == range_sum(s, from, to - 1) + s[to - 1] as int,
    decreases to - from,
{
    if from == to - 1 {
        // Explicit unfolding for base case.
        assert(range_sum(s, from + 1, to) == 0);
        assert(range_sum(s, from, to) == s[from] as int);
        assert(range_sum(s, from, to - 1) == 0);
        assert(s[to - 1] == s[from]);
    } else {
        range_sum_split_last(s, from + 1, to);
    }
}

/// When two sequences agree on [from, to), their range_sums are equal.
pub proof fn range_sum_prefix_eq(s1: Seq<i64>, s2: Seq<i64>, from: int, to: int)
    requires
        to <= s1.len(), to <= s2.len(),
        0 <= from,
        forall|j: int| from <= j < to ==> s1[j] == s2[j],
    ensures
        range_sum(s1, from, to) == range_sum(s2, from, to),
    decreases to - from,
{
    if from < to {
        range_sum_prefix_eq(s1, s2, from + 1, to);
    }
}

/// range_sum: advancing `from` by 1 peels off one element.
pub proof fn range_sum_step(s: Seq<i64>, from: int, to: int)
    requires 0 <= from < to, to <= s.len(),
    ensures range_sum(s, from, to) == s[from] as int + range_sum(s, from + 1, to),
{}

/// Updating s[idx] changes range_sum by the difference (when idx is in range).
pub proof fn range_sum_update(s: Seq<i64>, from: int, to: int, idx: int, new_val: i64)
    requires 0 <= from <= idx < to, to <= s.len(),
    ensures
        range_sum(s.update(idx, new_val), from, to)
            == range_sum(s, from, to) - s[idx] as int + new_val as int,
    decreases to - from,
{
    if from == idx {
        assert(s.update(idx, new_val)[from] == new_val);
        range_sum_rest_eq(s, s.update(idx, new_val), from + 1, to);
    } else {
        assert(s.update(idx, new_val)[from] == s[from]);
        range_sum_update(s, from + 1, to, idx, new_val);
    }
}

/// When two sequences agree on [from, to), their range_sums are equal.
proof fn range_sum_rest_eq(s1: Seq<i64>, s2: Seq<i64>, from: int, to: int)
    requires
        s1.len() == s2.len(),
        to <= s1.len(),
        forall|j: int| from <= j < to ==> s1[j] == s2[j],
    ensures
        range_sum(s1, from, to) == range_sum(s2, from, to),
    decreases to - from,
{
    if from < to {
        range_sum_rest_eq(s1, s2, from + 1, to);
    }
}


/// Updating one element changes seq_sum by the difference.
pub proof fn seq_sum_update(s: Seq<i64>, idx: int, new_val: i64)
    requires 0 <= idx < s.len(),
    ensures seq_sum(s.update(idx, new_val)) == seq_sum(s) - s[idx] as int + new_val as int,
    decreases s.len(),
{
    if s.len() == 1 {
        assert(s.update(idx, new_val).drop_last() =~= Seq::<i64>::empty());
        assert(s.drop_last() =~= Seq::<i64>::empty());
    } else if idx == s.len() - 1 {
        assert(s.update(idx, new_val).drop_last() =~= s.drop_last());
    } else {
        assert(s.update(idx, new_val).drop_last() =~= s.drop_last().update(idx, new_val));
        seq_sum_update(s.drop_last(), idx, new_val);
    }
}

/// range_sum of all non-negative values is non-negative.
pub proof fn range_sum_nonneg(s: Seq<i64>, from: int, to: int)
    requires
        0 <= from, to <= s.len(),
        forall|j: int| from <= j < to ==> #[trigger] s[j] >= 0,
    ensures
        range_sum(s, from, to) >= 0,
    decreases to - from,
{
    if from < to {
        range_sum_nonneg(s, from + 1, to);
    }
}

}
