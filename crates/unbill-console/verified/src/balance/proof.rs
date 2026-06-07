// Proof utilities for the balance settlement module.

use super::spec::*;
use vstd::prelude::*;

verus! {

/// Pushing a transaction adds its amount to the sum.
pub proof fn transaction_sum_push(s: Seq<TransactionSpec>, t: TransactionSpec)
    ensures
        transaction_sum(s.push(t)) == transaction_sum(s) + t.amount_cents,
{
    assert(s.push(t).drop_last() =~= s);
}

/// seq_sum distributes over push.
pub proof fn seq_sum_push(s: Seq<i64>, x: i64)
    ensures
        seq_sum(s.push(x)) == seq_sum(s) + x as int,
{
    assert(s.push(x).drop_last() =~= s);
}

}
