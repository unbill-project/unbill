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

// ---------------------------------------------------------------------------
// floor division lemmas
// ---------------------------------------------------------------------------

/// Lemma: floor(a * b / c) <= a when 0 <= b <= c and c > 0.
pub proof fn floor_div_le(a: int, b: int, c: int)
    requires
        a >= 0,
        b >= 0,
        b <= c,
        c > 0,
    ensures
        (a * b) / c <= a,
{
    assert(a * b <= a * c) by(nonlinear_arith)
        requires a >= 0, b <= c;
    assert(a * b >= 0) by(nonlinear_arith)
        requires a >= 0, b >= 0;
    // Use vstd's monotonicity lemma: x <= y && z > 0 ==> x/z <= y/z.
    vstd::arithmetic::div_mod::lemma_div_is_ordered(a * b, a * c, c);
    // (a*b)/c <= (a*c)/c. Now show (a*c)/c == a.
    // a*c == c*a, and c*a/c == a by definition.
    assert(a * c == c * a) by(nonlinear_arith);
    vstd::arithmetic::div_mod::lemma_div_multiples_vanish(a, c);
}

/// Lemma: floor(a * b / c) >= 0 when a >= 0, b >= 0, c > 0.
pub proof fn floor_div_nonneg(a: int, b: int, c: int)
    requires
        a >= 0,
        b >= 0,
        c > 0,
    ensures
        (a * b) / c >= 0,
{
    assert(a * b >= 0) by(nonlinear_arith)
        requires a >= 0, b >= 0;
}

// ---------------------------------------------------------------------------
// sum-of-floors lemmas
// ---------------------------------------------------------------------------

/// Helper spec: sum of floor amounts.
pub open spec fn floor_sum(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    total_weight: int,
) -> int
    decreases shares.len(),
{
    if shares.len() == 0 {
        0
    } else {
        floor_sum(shares.drop_last(), total_cents, total_weight)
            + (total_cents * shares.last().1 as int) / total_weight
    }
}

/// Lemma: floor_sum <= t * partial_weight / W.
/// This is the key inductive property. When partial_weight == W, gives floor_sum <= t.
pub proof fn floor_sum_le_proportional(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    total_weight: int,
)
    requires
        total_cents >= 0,
        total_weight > 0,
    ensures
        floor_sum(shares, total_cents, total_weight) * total_weight
            <= total_cents * spec_total_weight(shares),
    decreases shares.len(),
{
    if shares.len() > 0 {
        floor_sum_le_proportional(shares.drop_last(), total_cents, total_weight);
        let w = shares.last().1 as int;
        let fs_init = floor_sum(shares.drop_last(), total_cents, total_weight);
        let tw_init = spec_total_weight(shares.drop_last());
        let floor_last = (total_cents * w) / total_weight;
        // IH: fs_init * W <= t * tw_init
        // Need: (fs_init + floor_last) * W <= t * (tw_init + w)
        // i.e.: fs_init * W + floor_last * W <= t * tw_init + t * w
        // By IH: fs_init * W <= t * tw_init
        // By def of floor: floor_last * W <= t * w (since floor_last <= t*w/W means floor_last*W <= t*w)
        assert(floor_last * total_weight <= total_cents * w) by(nonlinear_arith)
            requires total_weight > 0, floor_last == (total_cents * w) / total_weight;
        assert((fs_init + floor_last) * total_weight <= total_cents * tw_init + total_cents * w) by(nonlinear_arith)
            requires fs_init * total_weight <= total_cents * tw_init,
                     floor_last * total_weight <= total_cents * w;
        assert(total_cents * tw_init + total_cents * w == total_cents * (tw_init + w)) by(nonlinear_arith);
    }
}

/// Lemma: floor_sum <= total_cents when total_weight == spec_total_weight(shares).
pub proof fn floor_sum_le_total(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    total_weight: int,
)
    requires
        total_cents >= 0,
        total_weight > 0,
        total_weight == spec_total_weight(shares),
    ensures
        floor_sum(shares, total_cents, total_weight) <= total_cents,
{
    floor_sum_le_proportional(shares, total_cents, total_weight);
    // fs * W <= t * W, so fs <= t.
    assert(floor_sum(shares, total_cents, total_weight) <= total_cents) by(nonlinear_arith)
        requires floor_sum(shares, total_cents, total_weight) * total_weight
                    <= total_cents * total_weight,
                 total_weight > 0;
}

/// Lemma: floor_sum >= 0.
pub proof fn floor_sum_nonneg(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    total_weight: int,
)
    requires
        total_cents >= 0,
        total_weight > 0,
    ensures
        floor_sum(shares, total_cents, total_weight) >= 0,
    decreases shares.len(),
{
    if shares.len() > 0 {
        floor_sum_nonneg(shares.drop_last(), total_cents, total_weight);
        floor_div_nonneg(total_cents, shares.last().1 as int, total_weight);
    }
}

/// Lemma: total rounding loss < n.
/// Proved by: floor_sum * W >= t * sum(w_i) - n * W + n (each floor loses < 1).
/// Equivalently: t * sum(w_i) - floor_sum * W < n * W.
pub proof fn floor_sum_loss_bound(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    total_weight: int,
)
    requires
        total_cents >= 0,
        total_weight > 0,
        shares.len() > 0,
    ensures
        total_cents * spec_total_weight(shares)
            - floor_sum(shares, total_cents, total_weight) * total_weight
            < shares.len() as int * total_weight,
        total_cents * spec_total_weight(shares)
            - floor_sum(shares, total_cents, total_weight) * total_weight
            >= 0,
    decreases shares.len(),
{
    if shares.len() == 1 {
        // Single element: loss = t*w - floor(t*w/W)*W ∈ [0, W).
        let w = shares.last().1 as int;
        let floor_val = (total_cents * w) / total_weight;
        assert(total_cents * w - floor_val * total_weight >= 0) by(nonlinear_arith)
            requires total_cents >= 0, w >= 0, total_weight > 0,
                     floor_val == (total_cents * w) / total_weight;
        assert(total_cents * w - floor_val * total_weight < total_weight) by(nonlinear_arith)
            requires total_weight > 0, floor_val == (total_cents * w) / total_weight,
                     total_cents >= 0, w >= 0;
        spec_total_weight_nonneg(shares.drop_last());
        assert(shares.drop_last().len() == 0);
        assert(spec_total_weight(shares.drop_last()) == 0);
        assert(floor_sum(shares.drop_last(), total_cents, total_weight) == 0);
    } else {
        floor_sum_loss_bound(shares.drop_last(), total_cents, total_weight);
        let w = shares.last().1 as int;
        let floor_last = (total_cents * w) / total_weight;
        // Per-term loss: t*w - floor_last * W ∈ [0, W).
        // This is just (t*w) % W, which is in [0, W).
        // The inductive step adds one term with loss < W.
        assert(total_cents * w - floor_last * total_weight >= 0) by(nonlinear_arith)
            requires total_cents >= 0, w >= 0, total_weight > 0,
                     floor_last == (total_cents * w) / total_weight;
        assert(total_cents * w - floor_last * total_weight < total_weight) by(nonlinear_arith)
            requires total_weight > 0, floor_last == (total_cents * w) / total_weight,
                     total_cents >= 0, w >= 0;
        spec_total_weight_nonneg(shares.drop_last());
        let tw_init = spec_total_weight(shares.drop_last());
        let fs_init = floor_sum(shares.drop_last(), total_cents, total_weight);
        // IH: t * tw_init - fs_init * W < (n-1) * W and >= 0.
        // New: t * (tw_init + w) - (fs_init + floor_last) * W
        //    = (t * tw_init - fs_init * W) + (t * w - floor_last * W)
        //    < (n-1) * W + W = n * W.
        assert(total_cents * (tw_init + w) - (fs_init + floor_last) * total_weight
            == (total_cents * tw_init - fs_init * total_weight)
                + (total_cents * w - floor_last * total_weight)) by(nonlinear_arith);
        assert((total_cents * tw_init - fs_init * total_weight)
                + (total_cents * w - floor_last * total_weight)
            < (shares.drop_last().len() as int) * total_weight + total_weight) by(nonlinear_arith)
            requires
                total_cents * tw_init - fs_init * total_weight < shares.drop_last().len() as int * total_weight,
                total_cents * w - floor_last * total_weight < total_weight,
                total_cents * tw_init - fs_init * total_weight >= 0,
                total_cents * w - floor_last * total_weight >= 0;
        assert(shares.drop_last().len() + 1 == shares.len());
        assert(spec_total_weight(shares) == tw_init + w);
        assert(floor_sum(shares, total_cents, total_weight) == fs_init + floor_last);
        // Connect all the pieces for the postcondition.
        assert(total_cents * (tw_init + w) == total_cents * tw_init + total_cents * w) by(nonlinear_arith);
        assert((fs_init + floor_last) * total_weight == fs_init * total_weight + floor_last * total_weight) by(nonlinear_arith);
        assert(total_cents * spec_total_weight(shares)
            - floor_sum(shares, total_cents, total_weight) * total_weight
            == (total_cents * tw_init - fs_init * total_weight)
                + (total_cents * w - floor_last * total_weight));
        assert((total_cents * tw_init - fs_init * total_weight)
                + (total_cents * w - floor_last * total_weight)
            < shares.drop_last().len() as int * total_weight + total_weight);
        let n = shares.len() as int;
        let n1 = shares.drop_last().len() as int;
        assert(n == n1 + 1);
        let il = total_cents * tw_init - fs_init * total_weight;
        let ll = total_cents * w - floor_last * total_weight;
        assert(total_cents * spec_total_weight(shares)
            - floor_sum(shares, total_cents, total_weight) * total_weight
            == il + ll);
        assert(il + ll < n1 * total_weight + total_weight);
        assert(n1 * total_weight + total_weight == n * total_weight) by(nonlinear_arith)
            requires n == n1 + 1;
    }
}

/// Lemma: remainder < n when total_weight == spec_total_weight(shares).
pub proof fn floor_sum_remainder_lt_n(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    total_weight: int,
)
    requires
        total_cents >= 0,
        total_weight > 0,
        total_weight == spec_total_weight(shares),
    ensures
        total_cents - floor_sum(shares, total_cents, total_weight) < shares.len(),
        total_cents - floor_sum(shares, total_cents, total_weight) >= 0,
{
    floor_sum_loss_bound(shares, total_cents, total_weight);
    // We have: t * W - floor_sum * W < n * W and >= 0.
    // Since total_weight == W: (t - floor_sum) * W < n * W.
    // Dividing by W > 0: t - floor_sum < n.
    assert((total_cents - floor_sum(shares, total_cents, total_weight)) * total_weight
        == total_cents * total_weight
            - floor_sum(shares, total_cents, total_weight) * total_weight) by(nonlinear_arith);
    assert((total_cents - floor_sum(shares, total_cents, total_weight)) * total_weight
        < shares.len() as int * total_weight);
    assert(total_cents - floor_sum(shares, total_cents, total_weight) < shares.len()) by(nonlinear_arith)
        requires
            (total_cents - floor_sum(shares, total_cents, total_weight)) * total_weight
                < shares.len() as int * total_weight,
            total_weight > 0;
    floor_sum_le_total(shares, total_cents, total_weight);
}

// ---------------------------------------------------------------------------
// modular index lemmas
// ---------------------------------------------------------------------------

/// Lemma: (base + r1) % n != (base + r2) % n when r1 != r2 and both < n.
pub proof fn mod_distinct(base: int, r1: int, r2: int, n: int)
    requires
        n > 0,
        0 <= r1 < n,
        0 <= r2 < n,
        r1 != r2,
    ensures
        (base + r1) % n != (base + r2) % n,
{
    let b = base % n;
    vstd::arithmetic::div_mod::lemma_mod_division_less_than_divisor(base, n);
    // 0 <= b < n, so 0 <= b + ri < 2n for ri in [0, n).
    // Compute (base + ri) % n via lemma_mod_adds:
    // (base + ri) % n depends on whether b + ri < n or not.
    // Case analysis on b + r1 and b + r2:
    vstd::arithmetic::div_mod::lemma_mod_adds(base, r1, n);
    vstd::arithmetic::div_mod::lemma_mod_adds(base, r2, n);

    // When a%n + b%n < n: (a+b)%n == a%n + b%n.
    // r1 < n, so r1%n == r1 (by small_mod). Same for r2.
    vstd::arithmetic::div_mod::lemma_small_mod(r1 as nat, n as nat);
    vstd::arithmetic::div_mod::lemma_small_mod(r2 as nat, n as nat);
    // So: if b + r1 < n: (base+r1)%n == b + r1.
    //     if b + r1 >= n: (base+r1)%n == b + r1 - n (since b+r1 < 2n).
    // Same pattern for r2.
    // In all cases: the results differ because r1 != r2.

    // Compute (base+r1)%n and (base+r2)%n via case analysis.
    // base = q*n + b where b = base%n, 0 <= b < n.
    // base+ri = q*n + b + ri.
    // If b+ri < n: (base+ri)%n = b+ri (since base+ri = q*n + (b+ri), 0 <= b+ri < n).
    // If b+ri >= n: (base+ri)%n = b+ri-n (since base+ri = (q+1)*n + (b+ri-n), 0 <= b+ri-n < n).
    let q = base / n;
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(base, n);
    assert(base == n * q + b);

    // Case: b + r1 < n
    if b + r1 < n {
        assert(base + r1 == q * n + (b + r1)) by(nonlinear_arith)
            requires base == n * q + b;
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod(base + r1, n, q, b + r1);
        // Now (base+r1)%n == b + r1.
    } else {
        assert(base + r1 == (q + 1) * n + (b + r1 - n)) by(nonlinear_arith)
            requires base == n * q + b;
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod(base + r1, n, q + 1, b + r1 - n);
        // Now (base+r1)%n == b + r1 - n.
    }

    if b + r2 < n {
        assert(base + r2 == q * n + (b + r2)) by(nonlinear_arith)
            requires base == n * q + b;
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod(base + r2, n, q, b + r2);
    } else {
        assert(base + r2 == (q + 1) * n + (b + r2 - n)) by(nonlinear_arith)
            requires base == n * q + b;
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod(base + r2, n, q + 1, b + r2 - n);
    }

    // In all four cases, the values differ because r1 != r2.
}

// ---------------------------------------------------------------------------
// floor_sum / amount_sum bridge
// ---------------------------------------------------------------------------

/// Lemma: when each amounts[j].1 == floor(t*w_j/W), amount_sum == floor_sum.
pub proof fn floor_sum_eq_amount_sum(
    shares: Seq<(u64, u32)>,
    amounts: Seq<(u64, i64)>,
    total_cents: int,
    total_weight: int,
)
    requires
        shares.len() == amounts.len(),
        total_weight > 0,
        total_cents >= 0,
        forall|j: int| 0 <= j < amounts.len() ==>
            amounts[j].1 as int == (total_cents * shares[j].1 as int) / total_weight,
    ensures
        amount_sum(amounts) == floor_sum(shares, total_cents, total_weight),
    decreases shares.len(),
{
    if shares.len() > 0 {
        floor_sum_eq_amount_sum(
            shares.drop_last(), amounts.drop_last(), total_cents, total_weight,
        );
        // amounts.last().1 == floor(t * shares.last().1 / W) == last term of floor_sum.
        assert(amounts.last().1 as int == (total_cents * shares.last().1 as int) / total_weight);
    }
}

}
// sirno:witness:formal-invariants:end
