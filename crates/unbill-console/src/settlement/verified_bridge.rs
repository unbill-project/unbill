// Bridge between production settlement types and the verified balance module.

use crate::model::UserId;

use unbill_console_verified::settlement::exec as v;

/// Convert a balance map to the verified format and compute settlement.
pub fn compute_from_balances_verified(balances: &[(UserId, i64)]) -> Vec<super::Transaction> {
    let user_ids: Vec<u128> = balances.iter().map(|(id, _)| id.to_u128()).collect();
    let balance_values: Vec<i64> = balances.iter().map(|(_, b)| *b).collect();

    let verified_transactions = v::compute_from_balances(&user_ids, &balance_values);

    verified_transactions
        .into_iter()
        .map(|t| super::Transaction {
            from_user_id: UserId::from_u128(t.from_user_id),
            to_user_id: UserId::from_u128(t.to_user_id),
            amount_cents: t.amount_cents,
        })
        .collect()
}
