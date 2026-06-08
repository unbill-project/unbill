// Verified effective bill filtering.
// Filters out superseded bills (those referenced in another bill's `prev`).

use unbill_model_verified::ledger::exec::*;
use unbill_model_verified::ledger::spec::*;
use vstd::hash_set::HashSetWithView;
use vstd::prelude::*;

verus! {

/// Find the index of a bill by its u128 ID. Linear scan.
/// Requires: the bill exists (guaranteed by bill_well_formed → has_bill).
fn find_bill_index_by_id(bills: &Vec<Bill>, target_id: u128) -> (idx: usize)
    requires
        exists|i: int| 0 <= i < bills.len() && (#[trigger] bills@[i]).id == target_id,
    ensures
        idx < bills.len(),
        bills@[idx as int].id == target_id,
{
    let mut k: usize = 0;
    while k < bills.len()
        invariant
            k <= bills.len(),
            forall|j: int| 0 <= j < k ==> (#[trigger] bills@[j]).id != target_id,
            exists|i: int| k <= i < bills.len() && (#[trigger] bills@[i]).id == target_id,
        decreases bills.len() - k,
    {
        if bills[k].id == target_id {
            return k;
        }
        k = k + 1;
    }
    proof { assert(false); }
    0
}

/// Filter effective bills from a ledger's bill list.
/// Returns indices of bills that are not superseded.
/// Uses vstd's HashSetWithView<usize> for O(1) membership checks.
pub fn filter_effective_indices(bills: &Vec<Bill>) -> (effective: Vec<usize>)
    requires
        // All prev references point to existing bills (from bill_well_formed).
        forall|i: int| #![trigger bills@[i]]
            0 <= i < bills.len() ==>
            forall|k: int| #![trigger bills@[i].prev@[k]]
                0 <= k < bills@[i].prev.len() ==>
                exists|j: int| 0 <= j < bills.len() && (#[trigger] bills@[j]).id == bills@[i].prev@[k],
        bills.len() <= i32::MAX as usize,
    ensures
        // All returned indices are valid and effective.
        forall|i: int| 0 <= i < effective.len() ==>
            (#[trigger] effective@[i]) < bills.len(),
        effective.len() <= bills.len(),
{
    // Build the set of superseded indices.
    proof { broadcast use vstd::std_specs::hash::axiom_usize_obeys_hash_table_key_model; }
    let mut superseded: HashSetWithView<usize> = HashSetWithView::new();

    let mut j: usize = 0;
    while j < bills.len()
        invariant
            j <= bills.len(),
            bills.len() <= i32::MAX as usize,
            forall|i: int| #![trigger bills@[i]]
                0 <= i < bills.len() ==>
                forall|k: int| #![trigger bills@[i].prev@[k]]
                    0 <= k < bills@[i].prev.len() ==>
                    exists|m: int| 0 <= m < bills.len() && (#[trigger] bills@[m]).id == bills@[i].prev@[k],
        decreases bills.len() - j,
    {
        let prev_len = bills[j].prev.len();
        let mut p: usize = 0;
        while p < prev_len
            invariant
                p <= prev_len,
                prev_len == bills@[j as int].prev.len(),
                j < bills.len(),
                forall|i: int| #![trigger bills@[i]]
                    0 <= i < bills.len() ==>
                    forall|k: int| #![trigger bills@[i].prev@[k]]
                        0 <= k < bills@[i].prev.len() ==>
                        exists|m: int| 0 <= m < bills.len() && (#[trigger] bills@[m]).id == bills@[i].prev@[k],
            decreases prev_len - p,
        {
            let target_id = bills[j].prev[p];
            // Trigger the quantifier: target_id == bills@[j].prev@[p].
            assert(bills@[j as int].prev@[p as int] == target_id);
            let idx = find_bill_index_by_id(bills, target_id);
            superseded.insert(idx);
            p = p + 1;
        }
        j = j + 1;
    }

    // Collect non-superseded indices.
    let mut effective: Vec<usize> = Vec::new();
    let mut i: usize = 0;
    while i < bills.len()
        invariant
            i <= bills.len(),
            effective.len() <= i,
            forall|k: int| 0 <= k < effective.len() ==>
                (#[trigger] effective@[k]) < bills.len(),
        decreases bills.len() - i,
    {
        if !superseded.contains(&i) {
            effective.push(i);
        }
        i = i + 1;
    }

    effective
}

}
