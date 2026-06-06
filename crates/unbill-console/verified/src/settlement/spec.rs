// Interface specifications for the settlement module.
// Contains only the spec fn definitions that appear in requires/ensures contracts.
// Proof utilities live in proof.rs.

use vstd::prelude::*;

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

}
// sirno:witness:formal-invariants:end
