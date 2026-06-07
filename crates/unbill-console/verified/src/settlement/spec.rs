// Interface specifications for the settlement module.
// Contains only the spec fn definitions that appear in requires/ensures contracts.
// Proof utilities live in proof.rs.
// ShareSpec and total_weight are imported from unbill-model-verified.

use unbill_model_verified::ledger::spec::*;
use vstd::prelude::*;

// sirno:witness:formal-invariants:begin
verus! {

/// The sum of amounts in a sequence.
pub open spec fn amount_sum(s: Seq<i64>) -> int
    decreases s.len(),
{
    if s.len() == 0 {
        0
    } else {
        amount_sum(s.drop_last()) + s.last() as int
    }
}

/// The floor amount for share i: floor(total_cents * w_i / total_weight).
pub open spec fn floor_amount(
    shares: Seq<ShareSpec>,
    total_cents: int,
    i: int,
) -> int {
    (total_cents * shares[i].weight as int) / total_weight(shares)
}

/// Precondition for split_shares.
pub open spec fn split_shares_requires(
    shares: Seq<ShareSpec>,
    total_cents: i64,
) -> bool {
    &&& shares.len() > 0
    &&& total_cents >= 0
    &&& total_cents <= i32::MAX as i64
    &&& shares.len() <= i32::MAX as int
    &&& total_weight(shares) > 0
    &&& total_weight(shares) <= u64::MAX as int
    &&& total_weight(shares) <= i64::MAX as int
}

/// Postcondition for split_shares.
pub open spec fn split_shares_ensures(
    shares: Seq<ShareSpec>,
    total_cents: i64,
    result: Seq<i64>,
) -> bool {
    // Conservation: amounts sum exactly to total_cents.
    &&& amount_sum(result) == total_cents as int
    // Length preservation.
    &&& result.len() == shares.len()
    // Fairness: every share is either floor or floor + 1 of the ideal proportion.
    &&& forall|i: int| 0 <= i < result.len() ==>
        #[trigger] result[i] as int >= floor_amount(shares, total_cents as int, i)
        && result[i] as int <= floor_amount(shares, total_cents as int, i) + 1
    // Non-negative: each amount >= 0.
    &&& forall|i: int| 0 <= i < result.len() ==> result[i] >= 0
}

}
// sirno:witness:formal-invariants:end
