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
// sirno:witness:formal-invariants:end
