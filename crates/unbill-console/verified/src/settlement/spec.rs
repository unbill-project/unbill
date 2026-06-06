// Formal specifications for the settlement module.
// Contains only Creusot logic functions, ghost helpers, and lemmas.
// See formal-verification and formal-invariants entries in the Sirno lake.

// sirno:witness:formal-invariants:begin
use creusot_std::prelude::*;

/// The sum of the second elements (amounts) in a sequence of (id, amount) pairs.
#[logic]
#[variant(s.len())]
pub fn amount_sum<Id>(s: Seq<(Id, i64)>) -> Int {
    pearlite! {
        if s.len() == 0 {
            0
        } else {
            amount_sum(s.subsequence(0, s.len() - 1)) + s[s.len() - 1].1@
        }
    }
}

/// Lemma: pushing an element adds its amount to the sum.
///
/// Proves: `amount_sum(s.push_back(x)) == amount_sum(s) + x.1@`
#[logic]
#[ensures(amount_sum(s.push_back(x)) == amount_sum(s) + x.1@)]
pub fn amount_sum_push_lemma<Id>(s: Seq<(Id, i64)>, x: (Id, i64)) {
    pearlite! {
        // s.push_back(x) == s.concat(Seq::singleton(x))
        // amount_sum(s.push_back(x)):
        //   len = s.len() + 1
        //   last = s.push_back(x)[s.len()] = x
        //   init = s.push_back(x).subsequence(0, s.len()) = s  (by ext_eq)
        // So amount_sum(s.push_back(x)) = amount_sum(s) + x.1@
        //
        // We assert ext_eq to help the solver see the init equals s.
        s.push_back(x).subsequence(0, s.len()).ext_eq(s);
    }
}

/// Lemma: updating one element changes the sum by the difference.
///
/// Proves: `amount_sum(s.set(i, (id, new_val))) == amount_sum(s) - s[i].1@ + new_val@`
#[logic]
#[variant(s.len())]
#[requires(0 <= i && i < s.len())]
#[ensures(amount_sum(s.set(i, (id, new_val))) == amount_sum(s) - s[i].1@ + new_val@)]
pub fn amount_sum_set_lemma<Id>(s: Seq<(Id, i64)>, i: Int, id: Id, new_val: i64) {
    pearlite! {
        if i == s.len() - 1 {
            // Updating the last element: init subsequence unchanged
            s.set(i, (id, new_val)).subsequence(0, s.len() - 1)
                .ext_eq(s.subsequence(0, s.len() - 1));
        } else {
            // Updating a non-last element: last element stays, recurse on init
            let init = s.subsequence(0, s.len() - 1);
            s.set(i, (id, new_val)).subsequence(0, s.len() - 1)
                .ext_eq(init.set(i, (id, new_val)));
            amount_sum_set_lemma(init, i, id, new_val);
        }
    }
}
// sirno:witness:formal-invariants:end
