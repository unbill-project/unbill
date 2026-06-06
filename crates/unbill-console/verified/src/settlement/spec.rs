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

/// The floor amount for share i: floor(total_cents * w_i / total_weight).
pub open spec fn floor_amount(
    shares: Seq<(u64, u32)>,
    total_cents: int,
    i: int,
) -> int {
    (total_cents * shares[i].1 as int) / spec_total_weight(shares)
}

/// Precondition for split_shares.
pub open spec fn split_shares_requires(
    shares: Seq<(u64, u32)>,
    total_cents: i64,
) -> bool {
    &&& shares.len() > 0
    &&& total_cents >= 0
    &&& total_cents <= i32::MAX as i64
    &&& shares.len() <= i32::MAX as int
    &&& spec_total_weight(shares) > 0
    &&& spec_total_weight(shares) <= u64::MAX as int
    &&& spec_total_weight(shares) <= i64::MAX as int
}

/// Postcondition for split_shares.
pub open spec fn split_shares_ensures(
    shares: Seq<(u64, u32)>,
    total_cents: i64,
    result: Seq<(u64, i64)>,
) -> bool {
    // Conservation: amounts sum exactly to total_cents.
    &&& amount_sum(result) == total_cents as int
    // Length preservation.
    &&& result.len() == shares.len()
    // Fairness: every share is either floor or floor + 1 of the ideal proportion.
    // No user is more than 1 cent away from their exact share.
    &&& forall|i: int| 0 <= i < result.len() ==>
        #[trigger] result[i].1 as int >= floor_amount(shares, total_cents as int, i)
        && result[i].1 as int <= floor_amount(shares, total_cents as int, i) + 1
}

}
// sirno:witness:formal-invariants:end
