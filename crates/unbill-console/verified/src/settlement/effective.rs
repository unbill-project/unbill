// Verified effective bill filtering.
// Filters out superseded bills (those referenced in another bill's `prev`).

use unbill_model_verified::ledger::exec::*;
use unbill_model_verified::ledger::spec::*;
use vstd::hash_set::HashSetWithView;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Exec-level superseded predicate (mirrors is_superseded but on Seq<Bill>)
// ---------------------------------------------------------------------------

/// Index `idx` is superseded: some OTHER bill references it in prev.
pub open spec fn idx_superseded(bills: Seq<Bill>, idx: int) -> bool {
    exists|j: int, k: int|
        0 <= j < bills.len() && j != idx
        && 0 <= k < bills[j].prev@.len()
        && #[trigger] bills[j].prev@[k] == bills[idx].id
}

// ---------------------------------------------------------------------------
// Helper: find bill index by ID
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Main filter function
// ---------------------------------------------------------------------------

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
        // Bill IDs are unique.
        forall|i: int, j: int|
            0 <= i < bills.len() && 0 <= j < bills.len() && i != j
            ==> (#[trigger] bills@[i]).id != (#[trigger] bills@[j]).id,
        // No bill references itself in prev.
        forall|i: int| 0 <= i < bills.len() ==>
            forall|k: int| 0 <= k < (#[trigger] bills@[i]).prev.len() ==>
                (#[trigger] bills@[i].prev@[k]) != bills@[i].id,
    ensures
        // All returned indices are valid.
        forall|i: int| 0 <= i < effective.len() ==>
            (#[trigger] effective@[i]) < bills.len(),
        effective.len() <= bills.len(),
        // Soundness: every returned index is NOT superseded.
        forall|i: int| 0 <= i < effective.len() ==>
            !idx_superseded(bills@, #[trigger] effective@[i] as int),
        // Completeness: every non-superseded index is returned.
        forall|idx: int| 0 <= idx < bills.len()
            && !idx_superseded(bills@, idx)
            ==> exists|k: int| 0 <= k < effective.len() && effective@[k] == idx as usize,
        // No duplicates.
        forall|i: int, j: int|
            0 <= i < effective.len() && 0 <= j < effective.len() && i != j
            ==> effective@[i] != effective@[j],
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
            forall|i2: int, j2: int|
                0 <= i2 < bills.len() && 0 <= j2 < bills.len() && i2 != j2
                ==> (#[trigger] bills@[i2]).id != (#[trigger] bills@[j2]).id,
            forall|i2: int| 0 <= i2 < bills.len() ==>
                forall|k: int| 0 <= k < (#[trigger] bills@[i2]).prev.len() ==>
                    (#[trigger] bills@[i2].prev@[k]) != bills@[i2].id,
            // D1 (soundness): if idx is in superseded, it was put there by some bill's prev in [0..j).
            forall|idx: usize| #[trigger] superseded@.contains(idx) ==> (
                (idx as int) < bills.len()
                && exists|jj: int, pp: int|
                    0 <= jj < j as int && 0 <= pp < bills@[jj].prev.len()
                    && bills@[jj].prev@[pp] == bills@[idx as int].id
            ),
            // D2 (completeness): all prev refs from [0..j) have their target in superseded.
            forall|jj: int, pp: int, idx: int|
                #![trigger bills@[jj].prev@[pp], bills@[idx].id]
                0 <= jj < j as int && 0 <= pp < bills@[jj].prev.len()
                && 0 <= idx < bills.len() && bills@[idx].id == bills@[jj].prev@[pp]
                ==> superseded@.contains(idx as usize),
        decreases bills.len() - j,
    {
        let prev_len = bills[j].prev.len();
        let mut p: usize = 0;
        while p < prev_len
            invariant
                p <= prev_len,
                prev_len == bills@[j as int].prev.len(),
                j < bills.len(),
                bills.len() <= i32::MAX as usize,
                forall|i: int| #![trigger bills@[i]]
                    0 <= i < bills.len() ==>
                    forall|k: int| #![trigger bills@[i].prev@[k]]
                        0 <= k < bills@[i].prev.len() ==>
                        exists|m: int| 0 <= m < bills.len() && (#[trigger] bills@[m]).id == bills@[i].prev@[k],
                forall|i2: int, j2: int|
                    0 <= i2 < bills.len() && 0 <= j2 < bills.len() && i2 != j2
                    ==> (#[trigger] bills@[i2]).id != (#[trigger] bills@[j2]).id,
                forall|i2: int| 0 <= i2 < bills.len() ==>
                    forall|k: int| 0 <= k < (#[trigger] bills@[i2]).prev.len() ==>
                        (#[trigger] bills@[i2].prev@[k]) != bills@[i2].id,
                // D1 extended: covers [0..j) fully + bill j's [0..p).
                forall|idx: usize| #[trigger] superseded@.contains(idx) ==> (
                    (idx as int) < bills.len()
                    && exists|jj: int, pp: int|
                        ((0 <= jj < j as int && 0 <= pp < bills@[jj].prev.len())
                         || (jj == j as int && 0 <= pp < p as int))
                        && bills@[jj].prev@[pp] == bills@[idx as int].id
                ),
                // D2 extended: covers [0..j) fully + bill j's [0..p).
                forall|jj: int, pp: int, idx: int|
                    #![trigger bills@[jj].prev@[pp], bills@[idx].id]
                    ((0 <= jj < j as int && 0 <= pp < bills@[jj].prev.len())
                     || (jj == j as int && 0 <= pp < p as int))
                    && 0 <= idx < bills.len() && bills@[idx].id == bills@[jj].prev@[pp]
                    ==> superseded@.contains(idx as usize),
            decreases prev_len - p,
        {
            let target_id = bills[j].prev[p];
            // Trigger the quantifier: target_id == bills@[j].prev@[p].
            assert(bills@[j as int].prev@[p as int] == target_id);
            let idx = find_bill_index_by_id(bills, target_id);

            // Prove D1 is maintained after insert.
            proof {
                let ghost old_set = superseded@;
                // idx < bills.len() from find_bill_index_by_id ensures.
                // Witness for new element: (jj=j, pp=p).
                assert(bills@[idx as int].id == bills@[j as int].prev@[p as int]);
            }

            superseded.insert(idx);

            proof {
                // D1: new element idx has witness (j, p). Old elements preserved.
                assert forall|x: usize| #[trigger] superseded@.contains(x)
                    implies (x as int) < bills.len()
                        && exists|jj: int, pp: int|
                            ((0 <= jj < j as int && 0 <= pp < bills@[jj].prev.len())
                             || (jj == j as int && 0 <= pp < (p + 1) as int))
                            && bills@[jj].prev@[pp] == bills@[x as int].id
                by {
                    if x == idx {
                        // Witness: jj = j, pp = p.
                        assert(j as int == j as int && (p as int) < (p + 1) as int);
                        assert(bills@[j as int].prev@[p as int] == bills@[idx as int].id);
                    }
                    // Old elements: their witness still satisfies the extended condition.
                }

                // D2: for the new case (jj=j, pp=p), idx is in the set.
                // For old cases: monotonicity of insert.
                assert forall|jj: int, pp: int, idx2: int|
                    #![trigger bills@[jj].prev@[pp], bills@[idx2].id]
                    ((0 <= jj < j as int && 0 <= pp < bills@[jj].prev.len())
                     || (jj == j as int && 0 <= pp < (p + 1) as int))
                    && 0 <= idx2 < bills.len() && bills@[idx2].id == bills@[jj].prev@[pp]
                    implies superseded@.contains(idx2 as usize)
                by {
                    if jj == j as int && pp == p as int {
                        // bills@[idx2].id == bills@[j].prev@[p] == target_id == bills@[idx].id
                        // By bill_ids_unique: idx2 == idx.
                        assert(bills@[idx2].id == bills@[idx as int].id);
                        if idx2 != idx as int {
                            assert(bills@[idx2].id != bills@[idx as int].id);
                        }
                        assert(idx2 == idx as int);
                        assert(superseded@.contains(idx));
                    }
                    // Old cases: idx2 was already in old set, still in new set.
                }
            }

            p = p + 1;
        }
        j = j + 1;
    }

    // After the loop: superseded@ characterizes exactly the superseded indices.
    // Connect to idx_superseded predicate.
    proof {
        // Soundness direction: superseded@.contains(idx) ==> idx_superseded(bills@, idx)
        assert forall|idx: usize| superseded@.contains(idx)
            implies idx_superseded(bills@, idx as int)
        by {
            // D1 gives: exists jj, pp with bills@[jj].prev@[pp] == bills@[idx].id
            // Extract witnesses via choose with explicit triggers.
            let jj = choose|jj: int|
                #![trigger bills@[jj]]
                0 <= jj < bills.len() as int
                && exists|pp: int| 0 <= pp < bills@[jj].prev.len()
                    && bills@[jj].prev@[pp] == bills@[idx as int].id;
            let pp = choose|pp: int|
                #![trigger bills@[jj].prev@[pp]]
                0 <= pp < bills@[jj].prev.len()
                && bills@[jj].prev@[pp] == bills@[idx as int].id;
            // no_self_prev: bills@[jj].prev@[pp] != bills@[jj].id
            assert(bills@[jj].prev@[pp] != bills@[jj].id);
            // So bills@[idx].id != bills@[jj].id, hence idx != jj by uniqueness.
            assert(bills@[idx as int].id != bills@[jj].id);
            if idx as int == jj {
                assert(bills@[idx as int].id == bills@[jj].id);
            }
            assert(idx as int != jj);
            // Witness for idx_superseded: j=jj, k=pp, jj != idx.
            assert(bills@[jj].prev@[pp] == bills@[idx as int].id);
        }

        // Completeness direction: idx_superseded(bills@, idx) ==> superseded@.contains(idx)
        assert forall|idx: int| 0 <= idx < bills.len()
            && idx_superseded(bills@, idx)
            implies superseded@.contains(idx as usize)
        by {
            // idx_superseded gives: exists j, k with j != idx and bills@[j].prev@[k] == bills@[idx].id
            let jj = choose|jj: int|
                #![trigger bills@[jj]]
                0 <= jj < bills.len() && jj != idx
                && exists|kk: int| 0 <= kk < bills@[jj].prev@.len()
                    && bills@[jj].prev@[kk] == bills@[idx].id;
            let kk = choose|kk: int|
                #![trigger bills@[jj].prev@[kk]]
                0 <= kk < bills@[jj].prev@.len()
                && bills@[jj].prev@[kk] == bills@[idx].id;
            // D2 with (jj, kk, idx): since jj < bills.len() == j after loop,
            // and bills@[idx].id == bills@[jj].prev@[kk], we get superseded@.contains(idx).
            assert(bills@[jj].prev@[kk] == bills@[idx].id);
            assert(bills@[idx].id == bills@[idx].id); // trigger for D2
        }
    }

    // Collect non-superseded indices.
    let mut effective: Vec<usize> = Vec::new();
    let mut i: usize = 0;
    while i < bills.len()
        invariant
            i <= bills.len(),
            effective.len() <= i,
            // All entries are valid indices < i.
            forall|k: int| 0 <= k < effective.len() ==>
                (#[trigger] effective@[k]) < i,
            // Soundness: all entries are not in superseded.
            forall|k: int| 0 <= k < effective.len() ==>
                !superseded@.contains(#[trigger] effective@[k]),
            // Completeness: all non-superseded indices in [0..i) are in effective.
            forall|idx: usize| #![trigger superseded@.contains(idx)]
                idx < i && !superseded@.contains(idx)
                ==> exists|k: int| 0 <= k < effective.len() && effective@[k] == idx,
            // No duplicates (follows from entries being < i and pushed in order).
            forall|k1: int, k2: int|
                0 <= k1 < effective.len() && 0 <= k2 < effective.len() && k1 != k2
                ==> effective@[k1] != effective@[k2],
        decreases bills.len() - i,
    {
        if !superseded.contains(&i) {
            let ghost old_effective = effective@;
            proof {
                // All existing entries < i, so i is distinct from all of them.
                assert forall|k1: int, k2: int|
                    0 <= k1 < effective@.len() + 1 && 0 <= k2 < effective@.len() + 1 && k1 != k2
                    implies effective@.push(i)[k1] != effective@.push(i)[k2]
                by {
                    if k1 == effective@.len() as int {
                        assert(effective@.push(i)[k1] == i);
                        assert(effective@.push(i)[k2] == effective@[k2]);
                        assert(effective@[k2] < i);
                    } else if k2 == effective@.len() as int {
                        assert(effective@.push(i)[k2] == i);
                        assert(effective@.push(i)[k1] == effective@[k1]);
                        assert(effective@[k1] < i);
                    } else {
                        assert(effective@.push(i)[k1] == effective@[k1]);
                        assert(effective@.push(i)[k2] == effective@[k2]);
                    }
                }
            }
            effective.push(i);
            proof {
                // Completeness: old witnesses still valid, plus new witness for idx == i.
                assert forall|idx: usize| #![trigger superseded@.contains(idx)]
                    idx < i + 1 && !superseded@.contains(idx)
                    implies exists|k: int| 0 <= k < effective.len() && effective@[k] == idx
                by {
                    if idx == i {
                        // We just pushed i. Witness: k = old_effective.len().
                        assert(effective@[old_effective.len() as int] == i);
                    } else {
                        // idx < i: old invariant gives witness k, and push preserves it.
                        let k = choose|k: int| 0 <= k < old_effective.len() && old_effective[k] == idx;
                        assert(effective@[k] == old_effective[k]);
                    }
                }
            }
        } else {
            proof {
                // superseded.contains(i), so idx == i case is vacuous.
                assert forall|idx: usize| #![trigger superseded@.contains(idx)]
                    idx < i + 1 && !superseded@.contains(idx)
                    implies exists|k: int| 0 <= k < effective.len() && effective@[k] == idx
                by {
                    // idx < i: use existing invariant. idx == i: superseded, so antecedent false.
                }
            }
        }
        i = i + 1;
    }

    proof {
        // Connect superseded membership to idx_superseded for final ensures.
        assert forall|k: int| 0 <= k < effective.len()
            implies !idx_superseded(bills@, #[trigger] effective@[k] as int)
        by {
            assert(!superseded@.contains(effective@[k]));
            // If idx_superseded held, completeness direction would give superseded@.contains — contradiction.
            if idx_superseded(bills@, effective@[k] as int) {
                assert(superseded@.contains(effective@[k]));
            }
        }

        assert forall|idx: int| 0 <= idx < bills.len()
            && !idx_superseded(bills@, idx)
            implies exists|k: int| 0 <= k < effective.len() && effective@[k] == idx as usize
        by {
            // !idx_superseded ==> !superseded@.contains (contrapositive of soundness direction)
            if superseded@.contains(idx as usize) {
                assert(idx_superseded(bills@, idx));
            }
            assert(!superseded@.contains(idx as usize));
            // Completeness of second loop gives the witness.
        }
    }

    effective
}

}
