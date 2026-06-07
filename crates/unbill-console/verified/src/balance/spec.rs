// Settlement balance specification.
// Models the greedy creditor-debtor matching and proves conservation.

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Spec types
// ---------------------------------------------------------------------------

/// A settlement transaction at the spec level.
pub struct TransactionSpec {
    pub from_user_id: Seq<u8>,
    pub to_user_id: Seq<u8>,
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

/// All elements are positive.
pub open spec fn all_positive_transactions(transactions: Seq<TransactionSpec>) -> bool {
    forall|i: int| 0 <= i < transactions.len() ==>
        (#[trigger] transactions[i]).amount_cents > 0
}

// ---------------------------------------------------------------------------
// Contract predicates
// ---------------------------------------------------------------------------

/// Precondition for settle: balances sum to zero, bounded.
pub open spec fn settle_requires(balances: Seq<i64>) -> bool {
    &&& seq_sum(balances) == 0
    &&& balances.len() <= i32::MAX as int
    &&& positive_sum(balances) <= i64::MAX as int
}

/// Postcondition: conservation — transaction total equals creditor total.
pub open spec fn settle_ensures(
    balances: Seq<i64>,
    transactions: Seq<TransactionSpec>,
) -> bool {
    // Conservation: total moved == total credits == total debts.
    &&& transaction_sum(transactions) == positive_sum(balances)
    // All amounts are positive (no zero-value or negative transactions).
    &&& all_positive_transactions(transactions)
}

}
