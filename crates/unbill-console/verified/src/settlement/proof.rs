// Proof utilities for the settlement module.
// total_weight lemmas imported from unbill-model-verified.
// ShareSpec imported from unbill-model-verified.

use super::spec::*;
use unbill_model_verified::ledger::proof::*;
use unbill_model_verified::ledger::spec::*;
use vstd::prelude::*;
use vstd::seq_lib::*;

// sirno:witness:formal-invariants:begin
verus! {

// ---------------------------------------------------------------------------
// seq_sum lemmas
// ---------------------------------------------------------------------------

pub proof fn seq_sum_push(s: Seq<i64>, x: i64)
    ensures
        seq_sum(s.push(x)) == seq_sum(s) + x as int,
{
    assert(s.push(x).drop_last() =~= s);
}

pub proof fn seq_sum_update(s: Seq<i64>, idx: int, new_val: i64)
    requires
        0 <= idx < s.len(),
    ensures
        seq_sum(s.update(idx, new_val)) == seq_sum(s) - s[idx] as int + new_val as int,
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

// ---------------------------------------------------------------------------
// floor division lemmas
// ---------------------------------------------------------------------------

pub proof fn floor_div_le(a: int, b: int, c: int)
    requires a >= 0, b >= 0, b <= c, c > 0,
    ensures (a * b) / c <= a,
{
    assert(a * b <= a * c) by(nonlinear_arith) requires a >= 0, b <= c;
    assert(a * b >= 0) by(nonlinear_arith) requires a >= 0, b >= 0;
    vstd::arithmetic::div_mod::lemma_div_is_ordered(a * b, a * c, c);
    assert(a * c == c * a) by(nonlinear_arith);
    vstd::arithmetic::div_mod::lemma_div_multiples_vanish(a, c);
}

pub proof fn floor_div_nonneg(a: int, b: int, c: int)
    requires a >= 0, b >= 0, c > 0,
    ensures (a * b) / c >= 0,
{
    assert(a * b >= 0) by(nonlinear_arith) requires a >= 0, b >= 0;
}

// ---------------------------------------------------------------------------
// sum-of-floors lemmas
// ---------------------------------------------------------------------------

pub open spec fn floor_sum(shares: Seq<ShareSpec>, total_cents: int, tw: int) -> int
    decreases shares.len(),
{
    if shares.len() == 0 {
        0
    } else {
        floor_sum(shares.drop_last(), total_cents, tw)
            + (total_cents * shares.last().weight as int) / tw
    }
}

pub proof fn floor_sum_le_proportional(shares: Seq<ShareSpec>, total_cents: int, tw: int)
    requires total_cents >= 0, tw > 0,
    ensures floor_sum(shares, total_cents, tw) * tw <= total_cents * total_weight(shares),
    decreases shares.len(),
{
    if shares.len() > 0 {
        floor_sum_le_proportional(shares.drop_last(), total_cents, tw);
        let w = shares.last().weight as int;
        let fs_init = floor_sum(shares.drop_last(), total_cents, tw);
        let tw_init = total_weight(shares.drop_last());
        let floor_last = (total_cents * w) / tw;
        assert(floor_last * tw <= total_cents * w) by(nonlinear_arith)
            requires tw > 0, floor_last == (total_cents * w) / tw;
        assert((fs_init + floor_last) * tw <= total_cents * tw_init + total_cents * w) by(nonlinear_arith)
            requires fs_init * tw <= total_cents * tw_init, floor_last * tw <= total_cents * w;
        assert(total_cents * tw_init + total_cents * w == total_cents * (tw_init + w)) by(nonlinear_arith);
    }
}

pub proof fn floor_sum_le_total(shares: Seq<ShareSpec>, total_cents: int, tw: int)
    requires total_cents >= 0, tw > 0, tw == total_weight(shares),
    ensures floor_sum(shares, total_cents, tw) <= total_cents,
{
    floor_sum_le_proportional(shares, total_cents, tw);
    assert(floor_sum(shares, total_cents, tw) <= total_cents) by(nonlinear_arith)
        requires floor_sum(shares, total_cents, tw) * tw <= total_cents * tw, tw > 0;
}

pub proof fn floor_sum_nonneg(shares: Seq<ShareSpec>, total_cents: int, tw: int)
    requires total_cents >= 0, tw > 0,
    ensures floor_sum(shares, total_cents, tw) >= 0,
    decreases shares.len(),
{
    if shares.len() > 0 {
        floor_sum_nonneg(shares.drop_last(), total_cents, tw);
        floor_div_nonneg(total_cents, shares.last().weight as int, tw);
    }
}

pub proof fn floor_sum_loss_bound(shares: Seq<ShareSpec>, total_cents: int, tw: int)
    requires total_cents >= 0, tw > 0, shares.len() > 0,
    ensures
        total_cents * total_weight(shares) - floor_sum(shares, total_cents, tw) * tw
            < shares.len() as int * tw,
        total_cents * total_weight(shares) - floor_sum(shares, total_cents, tw) * tw >= 0,
    decreases shares.len(),
{
    if shares.len() == 1 {
        let w = shares.last().weight as int;
        let fv = (total_cents * w) / tw;
        assert(total_cents * w - fv * tw >= 0) by(nonlinear_arith)
            requires total_cents >= 0, w >= 0, tw > 0, fv == (total_cents * w) / tw;
        assert(total_cents * w - fv * tw < tw) by(nonlinear_arith)
            requires tw > 0, fv == (total_cents * w) / tw, total_cents >= 0, w >= 0;
        total_weight_nonneg(shares.drop_last());
        assert(shares.drop_last().len() == 0);
        assert(total_weight(shares.drop_last()) == 0);
        assert(floor_sum(shares.drop_last(), total_cents, tw) == 0);
    } else {
        floor_sum_loss_bound(shares.drop_last(), total_cents, tw);
        let w = shares.last().weight as int;
        let fl = (total_cents * w) / tw;
        assert(total_cents * w - fl * tw >= 0) by(nonlinear_arith)
            requires total_cents >= 0, w >= 0, tw > 0, fl == (total_cents * w) / tw;
        assert(total_cents * w - fl * tw < tw) by(nonlinear_arith)
            requires tw > 0, fl == (total_cents * w) / tw, total_cents >= 0, w >= 0;
        total_weight_nonneg(shares.drop_last());
        let tw_init = total_weight(shares.drop_last());
        let fs_init = floor_sum(shares.drop_last(), total_cents, tw);
        assert(total_cents * (tw_init + w) - (fs_init + fl) * tw
            == (total_cents * tw_init - fs_init * tw) + (total_cents * w - fl * tw)) by(nonlinear_arith);
        assert((total_cents * tw_init - fs_init * tw) + (total_cents * w - fl * tw)
            < (shares.drop_last().len() as int) * tw + tw) by(nonlinear_arith)
            requires total_cents * tw_init - fs_init * tw < shares.drop_last().len() as int * tw,
                     total_cents * w - fl * tw < tw,
                     total_cents * tw_init - fs_init * tw >= 0,
                     total_cents * w - fl * tw >= 0;
        assert(shares.drop_last().len() + 1 == shares.len());
        assert(total_weight(shares) == tw_init + w);
        assert(floor_sum(shares, total_cents, tw) == fs_init + fl);
        assert(total_cents * (tw_init + w) == total_cents * tw_init + total_cents * w) by(nonlinear_arith);
        assert((fs_init + fl) * tw == fs_init * tw + fl * tw) by(nonlinear_arith);
        let n = shares.len() as int;
        let n1 = shares.drop_last().len() as int;
        assert(n == n1 + 1);
        assert(n1 * tw + tw == n * tw) by(nonlinear_arith) requires n == n1 + 1;
    }
}

pub proof fn floor_sum_remainder_lt_n(shares: Seq<ShareSpec>, total_cents: int, tw: int)
    requires total_cents >= 0, tw > 0, tw == total_weight(shares),
    ensures
        total_cents - floor_sum(shares, total_cents, tw) < shares.len(),
        total_cents - floor_sum(shares, total_cents, tw) >= 0,
{
    floor_sum_loss_bound(shares, total_cents, tw);
    assert((total_cents - floor_sum(shares, total_cents, tw)) * tw
        == total_cents * tw - floor_sum(shares, total_cents, tw) * tw) by(nonlinear_arith);
    assert((total_cents - floor_sum(shares, total_cents, tw)) * tw < shares.len() as int * tw);
    assert(total_cents - floor_sum(shares, total_cents, tw) < shares.len()) by(nonlinear_arith)
        requires (total_cents - floor_sum(shares, total_cents, tw)) * tw < shares.len() as int * tw, tw > 0;
    floor_sum_le_total(shares, total_cents, tw);
}

// ---------------------------------------------------------------------------
// modular index lemmas
// ---------------------------------------------------------------------------

pub proof fn mod_distinct(base: int, r1: int, r2: int, n: int)
    requires n > 0, 0 <= r1 < n, 0 <= r2 < n, r1 != r2,
    ensures (base + r1) % n != (base + r2) % n,
{
    let b = base % n;
    vstd::arithmetic::div_mod::lemma_mod_division_less_than_divisor(base, n);
    vstd::arithmetic::div_mod::lemma_mod_adds(base, r1, n);
    vstd::arithmetic::div_mod::lemma_mod_adds(base, r2, n);
    vstd::arithmetic::div_mod::lemma_small_mod(r1 as nat, n as nat);
    vstd::arithmetic::div_mod::lemma_small_mod(r2 as nat, n as nat);
    let q = base / n;
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(base, n);
    assert(base == n * q + b);
    if b + r1 < n {
        assert(base + r1 == q * n + (b + r1)) by(nonlinear_arith) requires base == n * q + b;
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod(base + r1, n, q, b + r1);
    } else {
        assert(base + r1 == (q + 1) * n + (b + r1 - n)) by(nonlinear_arith) requires base == n * q + b;
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod(base + r1, n, q + 1, b + r1 - n);
    }
    if b + r2 < n {
        assert(base + r2 == q * n + (b + r2)) by(nonlinear_arith) requires base == n * q + b;
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod(base + r2, n, q, b + r2);
    } else {
        assert(base + r2 == (q + 1) * n + (b + r2 - n)) by(nonlinear_arith) requires base == n * q + b;
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod(base + r2, n, q + 1, b + r2 - n);
    }
}

// ---------------------------------------------------------------------------
// floor_sum / seq_sum bridge
// ---------------------------------------------------------------------------

pub proof fn floor_sum_eq_seq_sum(
    shares: Seq<ShareSpec>, amounts: Seq<i64>, total_cents: int, tw: int,
)
    requires
        shares.len() == amounts.len(), tw > 0, total_cents >= 0,
        forall|j: int| 0 <= j < amounts.len() ==>
            amounts[j] as int == (total_cents * shares[j].weight as int) / tw,
    ensures
        seq_sum(amounts) == floor_sum(shares, total_cents, tw),
    decreases shares.len(),
{
    if shares.len() > 0 {
        floor_sum_eq_seq_sum(shares.drop_last(), amounts.drop_last(), total_cents, tw);
        assert(amounts.last() as int == (total_cents * shares.last().weight as int) / tw);
    }
}

// ---------------------------------------------------------------------------
// seq_sum bound lemma
// ---------------------------------------------------------------------------

/// If all elements >= 0 and sum == total, then each element <= total.
pub proof fn seq_sum_le_each(s: Seq<i64>, total: i64)
    requires
        seq_sum(s) == total as int,
        forall|i: int| 0 <= i < s.len() ==> s[i] >= 0,
    ensures
        forall|i: int| 0 <= i < s.len() ==> s[i] <= total,
{
    // Each s[i] >= 0. Sum of all others >= 0 (non-negative).
    // So s[i] = total - sum_of_others <= total.
    seq_sum_nonneg_all(s);
    seq_sum_each_le_sum(s);
}

/// seq_sum of non-negative elements is non-negative.
pub proof fn seq_sum_nonneg_all(s: Seq<i64>)
    requires forall|i: int| 0 <= i < s.len() ==> s[i] >= 0,
    ensures seq_sum(s) >= 0,
    decreases s.len(),
{
    if s.len() > 0 {
        seq_sum_nonneg_all(s.drop_last());
    }
}

/// Each element of a non-negative sequence is <= its sum.
proof fn seq_sum_each_le_sum(s: Seq<i64>)
    requires forall|i: int| 0 <= i < s.len() ==> s[i] >= 0,
    ensures forall|i: int| 0 <= i < s.len() ==> s[i] as int <= seq_sum(s),
    decreases s.len(),
{
    if s.len() > 0 {
        seq_sum_each_le_sum(s.drop_last());
        seq_sum_nonneg_all(s.drop_last());
        assert forall|i: int| 0 <= i < s.len()
            implies s[i] as int <= seq_sum(s)
        by {
            if i < s.len() - 1 {
                // s[i] == s.drop_last()[i] <= seq_sum(s.drop_last()) (IH)
                // seq_sum(s) == seq_sum(s.drop_last()) + s.last() >= seq_sum(s.drop_last())
                assert(s[i] == s.drop_last()[i]);
            } else {
                // i == s.len() - 1: s.last() = seq_sum(s) - seq_sum(s.drop_last())
                // seq_sum(s.drop_last()) >= 0, so s.last() <= seq_sum(s)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// transaction_sum lemma
// ---------------------------------------------------------------------------

pub proof fn transaction_sum_push(s: Seq<TransactionSpec>, t: TransactionSpec)
    ensures
        transaction_sum(s.push(t)) == transaction_sum(s) + t.amount_cents,
{
    assert(s.push(t).drop_last() =~= s);
}

// ---------------------------------------------------------------------------
// range_sum lemmas
// ---------------------------------------------------------------------------

pub proof fn range_sum_eq_seq_sum(s: Seq<i64>)
    ensures range_sum(s, 0, s.len() as int) == seq_sum(s),
    decreases s.len(),
{
    if s.len() > 0 {
        range_sum_split_last(s, 0, s.len() as int);
        range_sum_eq_seq_sum(s.drop_last());
        range_sum_prefix_eq(s, s.drop_last(), 0, s.len() as int - 1);
    }
}

proof fn range_sum_split_last(s: Seq<i64>, from: int, to: int)
    requires 0 <= from < to, to <= s.len(),
    ensures range_sum(s, from, to) == range_sum(s, from, to - 1) + s[to - 1] as int,
    decreases to - from,
{
    if from == to - 1 {
        assert(range_sum(s, from + 1, to) == 0);
        assert(range_sum(s, from, to) == s[from] as int);
        assert(range_sum(s, from, to - 1) == 0);
        assert(s[to - 1] == s[from]);
    } else {
        range_sum_split_last(s, from + 1, to);
    }
}

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

pub proof fn range_sum_step(s: Seq<i64>, from: int, to: int)
    requires 0 <= from < to, to <= s.len(),
    ensures range_sum(s, from, to) == s[from] as int + range_sum(s, from + 1, to),
{}

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

// ---------------------------------------------------------------------------
// overflow bound lemmas
// ---------------------------------------------------------------------------

/// If all elements of `a` >= corresponding elements of `b`, then seq_sum(a) >= seq_sum(b).
pub proof fn seq_sum_ge(a: Seq<i64>, b: Seq<i64>)
    requires
        a.len() == b.len(),
        forall|k: int| 0 <= k < a.len() ==> #[trigger] a[k] >= b[k],
    ensures
        seq_sum(a) >= seq_sum(b),
    decreases a.len(),
{
    if a.len() > 0 {
        assert forall|k: int| 0 <= k < a.drop_last().len()
            implies #[trigger] a.drop_last()[k] >= b.drop_last()[k]
        by {
            assert(a.drop_last()[k] == a[k]);
            assert(b.drop_last()[k] == b[k]);
        }
        seq_sum_ge(a.drop_last(), b.drop_last());
    }
}

/// If each element of `new` >= `old`, and total increase <= bound,
/// then each individual element increased by at most `bound`.
pub proof fn bound_elem_from_nonneg_increase(old: Seq<i64>, new: Seq<i64>, bound: int)
    requires
        old.len() == new.len(),
        forall|k: int| 0 <= k < old.len() ==> #[trigger] new[k] >= old[k],
        seq_sum(new) - seq_sum(old) <= bound,
    ensures
        forall|k: int| 0 <= k < old.len() ==> (#[trigger] new[k]) as int <= old[k] as int + bound,
    decreases old.len(),
{
    if old.len() > 0 {
        assert forall|k: int| 0 <= k < old.drop_last().len()
            implies #[trigger] new.drop_last()[k] >= old.drop_last()[k]
        by {
            assert(new.drop_last()[k] == new[k]);
            assert(old.drop_last()[k] == old[k]);
        }
        seq_sum_ge(new.drop_last(), old.drop_last());
        // seq_sum(new.dl) - seq_sum(old.dl) <= seq_sum(new) - seq_sum(old) <= bound
        // because new.last() >= old.last() implies the last pair consumed non-negative delta.
        bound_elem_from_nonneg_increase(old.drop_last(), new.drop_last(), bound);
        assert forall|k: int| 0 <= k < old.len()
            implies (#[trigger] new[k]) as int <= old[k] as int + bound
        by {
            if k < old.len() - 1 {
                assert(new[k] == new.drop_last()[k]);
                assert(old[k] == old.drop_last()[k]);
            } else {
                // last element: new.last() - old.last()
                //   = (seq_sum(new) - seq_sum(old)) - (seq_sum(new.dl) - seq_sum(old.dl))
                //   <= bound - 0 = bound
                // since seq_sum(new.dl) - seq_sum(old.dl) >= 0.
                seq_sum_ge(new.drop_last(), old.drop_last());
            }
        }
    }
}

/// Partial sum of a non-negative sequence is at most the total sum.
pub proof fn partial_sum_le_total(s: Seq<i64>, n: int)
    requires
        0 <= n <= s.len(),
        forall|k: int| 0 <= k < s.len() ==> (#[trigger] s[k]) >= 0,
    ensures
        seq_sum(s.subrange(0, n)) <= seq_sum(s),
    decreases s.len() - n,
{
    if n < s.len() {
        partial_sum_le_total(s.drop_last(), n);
        assert(s.drop_last().subrange(0, n) =~= s.subrange(0, n));
        // seq_sum(s) = seq_sum(s.drop_last()) + s.last() >= seq_sum(s.drop_last())
        //           >= seq_sum(s.drop_last().subrange(0,n)) = seq_sum(s.subrange(0,n))
    } else {
        assert(s.subrange(0, n) =~= s);
    }
}

// ---------------------------------------------------------------------------
// positive_sum update lemmas
// ---------------------------------------------------------------------------

/// If all transactions are positive and their sum is 0, there are no transactions.
pub proof fn no_transactions_if_sum_zero(ts: Seq<TransactionSpec>)
    requires
        all_positive_transactions(ts),
        transaction_sum(ts) == 0,
    ensures
        ts.len() == 0,
    decreases ts.len(),
{
    if ts.len() > 0 {
        // transaction_sum(ts) = transaction_sum(ts.drop_last()) + ts.last().amount_cents
        // ts.last().amount_cents > 0 (from all_positive_transactions)
        // So transaction_sum(ts.drop_last()) < transaction_sum(ts) = 0.
        // But we need transaction_sum(ts.drop_last()) >= 0 for contradiction.
        // Prove by induction: all-positive ⟹ transaction_sum >= 0.
        transaction_sum_nonneg_if_all_positive(ts.drop_last());
        assert(false);
    }
}

/// If all transactions have positive amounts, their sum is non-negative.
proof fn transaction_sum_nonneg_if_all_positive(ts: Seq<TransactionSpec>)
    requires all_positive_transactions(ts),
    ensures transaction_sum(ts) >= 0,
    decreases ts.len(),
{
    if ts.len() > 0 {
        assert forall|i: int| 0 <= i < ts.drop_last().len()
            implies (#[trigger] ts.drop_last()[i]).amount_cents > 0
        by { assert(ts.drop_last()[i] == ts[i]); }
        transaction_sum_nonneg_if_all_positive(ts.drop_last());
    }
}

/// Settlement idempotence: if balances are all zero, settlement
/// produces no transactions.  This means settling a second time
/// after "executing" a balanced settlement yields nothing.
pub proof fn settlement_idempotent(
    balances: Seq<i64>,
    transactions: Seq<TransactionSpec>,
)
    requires
        settle_ensures(balances, transactions),
        forall|i: int| 0 <= i < balances.len() ==> (#[trigger] balances[i]) == 0i64,
    ensures
        transactions.len() == 0,
{
    positive_sum_all_zero(balances);
    // positive_sum == 0 ⟹ transaction_sum == 0 ⟹ no transactions.
    no_transactions_if_sum_zero(transactions);
}

/// positive_sum is always non-negative.
pub proof fn positive_sum_nonneg(s: Seq<i64>)
    ensures positive_sum(s) >= 0,
    decreases s.len(),
{
    if s.len() > 0 {
        positive_sum_nonneg(s.drop_last());
    }
}

/// positive_sum of a sequence where all elements are zero is zero.
pub proof fn positive_sum_all_zero(s: Seq<i64>)
    requires forall|k: int| 0 <= k < s.len() ==> (#[trigger] s[k]) == 0i64,
    ensures positive_sum(s) == 0,
    decreases s.len(),
{
    if s.len() > 0 {
        assert forall|k: int| 0 <= k < s.drop_last().len()
            implies (#[trigger] s.drop_last()[k]) == 0i64
        by { assert(s.drop_last()[k] == s[k]); }
        positive_sum_all_zero(s.drop_last());
    }
}

/// Increasing one element increases positive_sum by at most the delta.
pub proof fn positive_sum_update_add(s: Seq<i64>, idx: int, new_val: i64)
    requires
        0 <= idx < s.len(),
        new_val as int >= s[idx] as int,
    ensures
        positive_sum(s.update(idx, new_val))
            <= positive_sum(s) + (new_val as int - s[idx] as int),
    decreases s.len(),
{
    let delta = new_val as int - s[idx] as int;
    if s.len() == 1 {
        assert(s.update(idx, new_val).drop_last() =~= Seq::<i64>::empty());
        assert(s.drop_last() =~= Seq::<i64>::empty());
    } else if idx == s.len() - 1 {
        // Last element changed.
        assert(s.update(idx, new_val).drop_last() =~= s.drop_last());
        // positive_sum(s.update) = positive_sum(s.drop_last()) + max(0, new_val)
        // positive_sum(s) = positive_sum(s.drop_last()) + max(0, s[idx])
        // Change = max(0, new_val) - max(0, s[idx]) ≤ new_val - s[idx] = delta.
    } else {
        // Element before the last; recurse on drop_last.
        assert(s.update(idx, new_val).drop_last() =~= s.drop_last().update(idx, new_val));
        assert(s.update(idx, new_val).last() == s.last());
        positive_sum_update_add(s.drop_last(), idx, new_val);
    }
}

/// Decreasing one element does not increase positive_sum.
pub proof fn positive_sum_update_sub(s: Seq<i64>, idx: int, new_val: i64)
    requires
        0 <= idx < s.len(),
        new_val as int <= s[idx] as int,
    ensures
        positive_sum(s.update(idx, new_val)) <= positive_sum(s),
    decreases s.len(),
{
    if s.len() == 1 {
        assert(s.update(idx, new_val).drop_last() =~= Seq::<i64>::empty());
        assert(s.drop_last() =~= Seq::<i64>::empty());
    } else if idx == s.len() - 1 {
        assert(s.update(idx, new_val).drop_last() =~= s.drop_last());
    } else {
        assert(s.update(idx, new_val).drop_last() =~= s.drop_last().update(idx, new_val));
        assert(s.update(idx, new_val).last() == s.last());
        positive_sum_update_sub(s.drop_last(), idx, new_val);
    }
}

}
// sirno:witness:formal-invariants:end
