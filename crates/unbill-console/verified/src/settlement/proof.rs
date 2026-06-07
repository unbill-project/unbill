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
// amount_sum lemmas
// ---------------------------------------------------------------------------

pub proof fn amount_sum_push_lemma(s: Seq<i64>, x: i64)
    ensures
        amount_sum(s.push(x)) == amount_sum(s) + x as int,
{
    assert(s.push(x).drop_last() =~= s);
}

pub proof fn amount_sum_set_lemma(s: Seq<i64>, idx: int, new_val: i64)
    requires
        0 <= idx < s.len(),
    ensures
        amount_sum(s.update(idx, new_val)) == amount_sum(s) - s[idx] as int + new_val as int,
    decreases s.len(),
{
    if s.len() == 1 {
        assert(s.update(idx, new_val).drop_last() =~= Seq::<i64>::empty());
        assert(s.drop_last() =~= Seq::<i64>::empty());
    } else if idx == s.len() - 1 {
        assert(s.update(idx, new_val).drop_last() =~= s.drop_last());
    } else {
        assert(s.update(idx, new_val).drop_last() =~= s.drop_last().update(idx, new_val));
        amount_sum_set_lemma(s.drop_last(), idx, new_val);
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
// floor_sum / amount_sum bridge
// ---------------------------------------------------------------------------

pub proof fn floor_sum_eq_amount_sum(
    shares: Seq<ShareSpec>, amounts: Seq<i64>, total_cents: int, tw: int,
)
    requires
        shares.len() == amounts.len(), tw > 0, total_cents >= 0,
        forall|j: int| 0 <= j < amounts.len() ==>
            amounts[j] as int == (total_cents * shares[j].weight as int) / tw,
    ensures
        amount_sum(amounts) == floor_sum(shares, total_cents, tw),
    decreases shares.len(),
{
    if shares.len() > 0 {
        floor_sum_eq_amount_sum(shares.drop_last(), amounts.drop_last(), total_cents, tw);
        assert(amounts.last() as int == (total_cents * shares.last().weight as int) / tw);
    }
}

}
// sirno:witness:formal-invariants:end
