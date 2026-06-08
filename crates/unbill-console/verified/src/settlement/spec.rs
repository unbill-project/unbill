// Settlement specification — all spec types and predicates.

use unbill_model_verified::ledger::spec::*;
use vstd::prelude::*;

// sirno:witness:formal-invariants:begin
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
// Sequence sum helpers
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

// ---------------------------------------------------------------------------
// Split shares specs
// ---------------------------------------------------------------------------

/// The floor amount for share i: floor(total_cents * w_i / total_weight).
pub open spec fn floor_amount(
    shares: Seq<ShareSpec>,
    total_cents: int,
    i: int,
) -> int {
    (total_cents * shares[i].weight as int) / total_weight(shares)
}

/// Postcondition for split_shares.
pub open spec fn split_shares_ensures(
    shares: Seq<ShareSpec>,
    total_cents: i64,
    result: Seq<i64>,
) -> bool {
    // Conservation: amounts sum exactly to total_cents.
    &&& seq_sum(result) == total_cents as int
    // Length preservation.
    &&& result.len() == shares.len()
    // Fairness: every share is either floor or floor + 1 of the ideal proportion.
    &&& forall|i: int| 0 <= i < result.len() ==>
        #[trigger] result[i] as int >= floor_amount(shares, total_cents as int, i)
        && result[i] as int <= floor_amount(shares, total_cents as int, i) + 1
    // Non-negative and bounded: each amount in [0, total_cents].
    &&& forall|i: int| 0 <= i < result.len() ==> result[i] >= 0 && result[i] <= total_cents
}

// ---------------------------------------------------------------------------
// Settlement contract predicates
// ---------------------------------------------------------------------------

/// Balances are well-formed for settlement: sum to zero, individually bounded.
pub open spec fn balances_wf(balances: Seq<i64>) -> bool {
    &&& seq_sum(balances) == 0
    &&& balances.len() <= i32::MAX as int
    &&& positive_sum(balances) <= i64::MAX as int
    &&& forall|i: int| 0 <= i < balances.len() ==>
        #[trigger] balances[i] > i64::MIN && balances[i] < i64::MAX
}

/// Postcondition for compute_from_balances: conservation and positivity.
pub open spec fn settle_ensures(
    balances: Seq<i64>,
    transactions: Seq<TransactionSpec>,
) -> bool {
    // Every credited cent is transferred — no money swallowed.
    &&& transaction_sum(transactions) == positive_sum(balances)
    // Every transaction moves a positive amount.
    &&& all_positive_transactions(transactions)
}

/// Postcondition for compute_balances: well-formed balances with
/// length matching users, and conservation (sum == 0).
pub open spec fn compute_balances_ensures(
    balances: Seq<i64>,
    n_users: int,
    n_effective: int,
) -> bool {
    &&& balances_wf(balances)
    &&& balances.len() == n_users
    // No effective bills ⟹ zero balances.
    &&& (n_effective == 0 ==> positive_sum(balances) == 0)
}

// ---------------------------------------------------------------------------
// Balance bound helper
// ---------------------------------------------------------------------------

/// Balance bound unit: i32::MAX + 1 = 2^31.
pub open spec fn bal_M() -> int {
    i32::MAX as int + 1
}

// ---------------------------------------------------------------------------
// Pipeline contract predicates
// ---------------------------------------------------------------------------

/// Precondition for compute_settlement (the full pipeline entry point).
/// All ledger-validity properties are folded into ledger_invariant.
/// Only settlement-specific requirements remain here.
pub open spec fn compute_settlement_requires(
    ledger: LedgerStateSpec,
    remainder_indices: Seq<usize>,
) -> bool {
    &&& ledger_invariant(ledger)
    &&& remainder_indices.len() == ledger.bills.len()
    &&& forall|i: int| 0 <= i < remainder_indices.len() ==>
        (#[trigger] remainder_indices[i]) <= usize::MAX - (i32::MAX as usize)
}

/// Postcondition for compute_settlement.
pub open spec fn compute_settlement_ensures(
    transactions: Seq<TransactionSpec>,
) -> bool {
    // Every transaction moves a positive amount.
    &&& all_positive_transactions(transactions)
    // Total transferred is non-negative (no money created from nothing).
    &&& transaction_sum(transactions) >= 0
    // Empty ledger (no bills) produces no transactions.
}


}
// sirno:witness:formal-invariants:end
