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

/// Precondition for greedy matching: balances sum to zero, bounded.
pub open spec fn settle_requires(balances: Seq<i64>) -> bool {
    &&& seq_sum(balances) == 0
    &&& balances.len() <= i32::MAX as int
    &&& positive_sum(balances) <= i64::MAX as int
    &&& forall|i: int| 0 <= i < balances.len() ==>
        #[trigger] balances[i] > i64::MIN && balances[i] < i64::MAX
}

/// Postcondition for greedy matching: conservation and positivity.
pub open spec fn settle_ensures(
    balances: Seq<i64>,
    transactions: Seq<TransactionSpec>,
) -> bool {
    &&& transaction_sum(transactions) == positive_sum(balances)
    &&& all_positive_transactions(transactions)
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
/// Expressed over LedgerStateSpec + exec-level Vec lengths so both
/// spec reasoning and exec callers can use it.
pub open spec fn compute_settlement_requires(
    ledger: LedgerStateSpec,
    bills_exec_len: int,
    users_exec_len: int,
    remainder_indices: Seq<usize>,
) -> bool {
    // Ledger invariant.
    &&& ledger_invariant(ledger)
    // Remainder indices match bills.
    &&& remainder_indices.len() == bills_exec_len
    // All prev references point to existing bills (exec-level).
    &&& forall|i: int| #![trigger ledger.bills[i]]
        0 <= i < ledger.bills.len() ==>
        forall|k: int| #![trigger ledger.bills[i].prev[k]]
            0 <= k < ledger.bills[i].prev.len() ==>
            exists|j: int| 0 <= j < ledger.bills.len()
                && (#[trigger] ledger.bills[j]).id == ledger.bills[i].prev[k]
    // User existence for payers.
    &&& forall|i: int, s: int| #![trigger ledger.bills[i].payers[s]]
        0 <= i < ledger.bills.len() && 0 <= s < ledger.bills[i].payers.len() ==>
        exists|k: int| 0 <= k < ledger.users.len()
            && (#[trigger] ledger.users[k]).user_id == ledger.bills[i].payers[s].user_id
    // User existence for payees.
    &&& forall|i: int, s: int| #![trigger ledger.bills[i].payees[s]]
        0 <= i < ledger.bills.len() && 0 <= s < ledger.bills[i].payees.len() ==>
        exists|k: int| 0 <= k < ledger.users.len()
            && (#[trigger] ledger.users[k]).user_id == ledger.bills[i].payees[s].user_id
    // Overflow bounds.
    &&& users_exec_len <= i32::MAX as int
    &&& bills_exec_len <= i32::MAX as int
    // split_shares preconditions.
    &&& forall|i: int| #![trigger ledger.bills[i]]
        0 <= i < ledger.bills.len() ==>
        split_shares_requires(ledger.bills[i].payers, ledger.bills[i].amount_cents)
        && split_shares_requires(ledger.bills[i].payees, ledger.bills[i].amount_cents)
    // Remainder indices bounded for split_shares remainder loop.
    &&& forall|i: int| 0 <= i < remainder_indices.len() ==>
        (#[trigger] remainder_indices[i]) <= usize::MAX - (i32::MAX as usize)
}

}
// sirno:witness:formal-invariants:end
