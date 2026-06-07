// Bridge between production settlement types and the verified balance module.
// Converts balances to the verified format, calls verified compute_from_balances,
// and converts the result back to production Transaction types.

use crate::model::UserId;

use unbill_console_verified::balance::exec as v;

/// Convert a balance map to the verified format and compute settlement.
/// Returns verified transactions converted back to production types.
pub fn compute_from_balances_verified(balances: &[(UserId, i64)]) -> Vec<super::Transaction> {
    // Separate user_ids and balance values for the verified function.
    let user_ids: Vec<Vec<u8>> = balances
        .iter()
        .map(|(id, _)| id.to_string().into_bytes())
        .collect();
    let balance_values: Vec<i64> = balances.iter().map(|(_, b)| *b).collect();

    let verified_transactions = v::compute_from_balances(&user_ids, &balance_values);

    // Convert verified Transaction → production Transaction.
    verified_transactions
        .into_iter()
        .map(|t| {
            let from_str = String::from_utf8(t.from_user_id)
                .expect("verified bridge: invalid UTF-8 in from_user_id");
            let to_str = String::from_utf8(t.to_user_id)
                .expect("verified bridge: invalid UTF-8 in to_user_id");
            super::Transaction {
                from_user_id: UserId::from_string(&from_str)
                    .expect("verified bridge: invalid UserId in from_user_id"),
                to_user_id: UserId::from_string(&to_str)
                    .expect("verified bridge: invalid UserId in to_user_id"),
                amount_cents: t.amount_cents,
            }
        })
        .collect()
}
