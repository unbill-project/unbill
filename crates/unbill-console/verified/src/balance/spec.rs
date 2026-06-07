// Settlement balance specification — all spec functions live here.

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Spec types
// ---------------------------------------------------------------------------

/// A settlement transaction at the spec level.
pub struct TransactionSpec {
    pub from_user_id: u128,
    pub to_user_id: u128,
    pub amount_cents: int,
}

// ---------------------------------------------------------------------------
// Helper specs
// ---------------------------------------------------------------------------

/// Sum of all elements in a sequence of i64.
pub open spec fn seq_sum(s: Seq<i64>) -> int
    decreases s.len(),
{
    if s.len() == 0 {
        0
    } else {
        seq_sum(s.drop_last()) + s.last() as int
    }
}

/// Sum of positive balances (creditor total).
pub open spec fn positive_sum(balances: Seq<i64>) -> int
    decreases balances.len(),
{
    if balances.len() == 0 {
        0
    } else if balances.last() > 0 {
        positive_sum(balances.drop_last()) + balances.last() as int
    } else {
        positive_sum(balances.drop_last())
    }
}

/// Sum of transaction amounts.
pub open spec fn transaction_sum(transactions: Seq<TransactionSpec>) -> int
    decreases transactions.len(),
{
    if transactions.len() == 0 {
        0
    } else {
        transaction_sum(transactions.drop_last()) + transactions.last().amount_cents
    }
}

/// All transactions have positive amounts.
pub open spec fn all_positive_transactions(transactions: Seq<TransactionSpec>) -> bool {
    forall|i: int| 0 <= i < transactions.len() ==>
        (#[trigger] transactions[i]).amount_cents > 0
}

/// Sum of elements in a range [from, to).
pub open spec fn range_sum(s: Seq<i64>, from: int, to: int) -> int
    decreases to - from,
{
    if from >= to {
        0
    } else {
        s[from] as int + range_sum(s, from + 1, to)
    }
}

/// Precondition for accumulate_bill.
pub open spec fn accumulate_requires(
    balances: Seq<i64>,
    payer_total: int,
    payee_total: int,
) -> bool {
    // Conservation from split_shares: payer amounts sum == payee amounts sum.
    &&& payer_total == payee_total
    // Balance sum starts at 0.
    &&& seq_sum(balances) == 0
}

/// Postcondition for accumulate_bill: sum still 0.
pub open spec fn accumulate_ensures(balances: Seq<i64>) -> bool {
    seq_sum(balances) == 0
}

// ---------------------------------------------------------------------------
// Contract predicates
// ---------------------------------------------------------------------------

/// Precondition for settle: balances sum to zero, bounded.
pub open spec fn settle_requires(balances: Seq<i64>) -> bool {
    &&& seq_sum(balances) == 0
    &&& balances.len() <= i32::MAX as int
    &&& positive_sum(balances) <= i64::MAX as int
    // Each value bounded to prevent overflow on negation.
    &&& forall|i: int| 0 <= i < balances.len() ==>
        #[trigger] balances[i] > i64::MIN && balances[i] < i64::MAX
}

/// Postcondition: conservation and positivity.
pub open spec fn settle_ensures(
    balances: Seq<i64>,
    transactions: Seq<TransactionSpec>,
) -> bool {
    // Conservation: total moved == total credits.
    &&& transaction_sum(transactions) == positive_sum(balances)
    // All amounts are positive.
    &&& all_positive_transactions(transactions)
}

}
