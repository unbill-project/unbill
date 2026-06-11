// Layer 1: Proofs that each transition preserves ledger_invariant,
// plus bridge lemmas for connecting runtime Vec to spec Seq through View.

use super::spec::*;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Bridge lemmas: map distributes over push
// ---------------------------------------------------------------------------

/// seq.push(x).map(f) =~= seq.map(f).push(f(seq.len(), x))
/// where f: spec_fn(int, A) -> B (Verus Seq::map takes index + value).
pub proof fn seq_map_push<A, B>(seq: Seq<A>, x: A, f: spec_fn(int, A) -> B)
    ensures
        seq.push(x).map(f) =~= seq.map(f).push(f(seq.len() as int, x)),
{
    let pushed = seq.push(x);
    let mapped_pushed = pushed.map(f);
    let mapped_orig = seq.map(f);
    assert forall|i: int| 0 <= i < mapped_pushed.len()
        implies (#[trigger] mapped_pushed[i]) == mapped_orig.push(f(seq.len() as int, x))[i]
    by {
        if i < seq.len() {
            assert(mapped_pushed[i] == f(i, pushed[i]));
            assert(pushed[i] == seq[i]);
            assert(mapped_orig[i] == f(i, seq[i]));
        } else {
            assert(i == seq.len());
            assert(mapped_pushed[i] == f(i, x));
        }
    }
}


// ---------------------------------------------------------------------------
// total_weight lemmas
// ---------------------------------------------------------------------------

/// total_weight distributes over push.
pub proof fn total_weight_push(s: Seq<ShareSpec>, x: ShareSpec)
    ensures
        total_weight(s.push(x)) == total_weight(s) + x.weight as int,
{
    assert(s.push(x).drop_last() =~= s);
}

/// total_weight is always non-negative.
pub proof fn total_weight_nonneg(shares: Seq<ShareSpec>)
    ensures
        total_weight(shares) >= 0,
    decreases shares.len(),
{
    if shares.len() > 0 {
        total_weight_nonneg(shares.drop_last());
    }
}

/// Each individual weight is <= total weight.
pub proof fn total_weight_includes_each(shares: Seq<ShareSpec>, idx: int)
    requires
        0 <= idx < shares.len(),
    ensures
        shares[idx].weight as int <= total_weight(shares),
    decreases shares.len(),
{
    if shares.len() == 1 {
        assert(shares.drop_last() =~= Seq::<ShareSpec>::empty());
    } else if idx == shares.len() - 1 {
        total_weight_nonneg(shares.drop_last());
    } else {
        total_weight_includes_each(shares.drop_last(), idx);
    }
}

/// Partial weight sum is <= total weight.
pub proof fn total_weight_partial_le(shares: Seq<ShareSpec>, n: int)
    requires
        0 <= n <= shares.len(),
    ensures
        total_weight(shares.subrange(0, n)) <= total_weight(shares),
    decreases shares.len(),
{
    if n == shares.len() {
        assert(shares.subrange(0, n) =~= shares);
    } else if shares.len() > 0 {
        total_weight_partial_le(shares.drop_last(), n);
        assert(shares.subrange(0, n) =~= shares.drop_last().subrange(0, n));
    }
}

// ---------------------------------------------------------------------------
// Transition preservation
// ---------------------------------------------------------------------------

pub proof fn init_preserves(
    post: LedgerStateSpec,
    ledger_id: u128,
    name: Seq<u8>,
    currency: Seq<u8>,
    created_at: i64,
)
    requires init(post, ledger_id, name, currency, created_at),
    ensures ledger_invariant(post),
{}

pub proof fn add_user_preserves(
    pre: LedgerStateSpec, post: LedgerStateSpec, user: UserSpec,
)
    requires
        ledger_invariant(pre),
        add_user(pre, post, user),
    ensures
        ledger_invariant(post),
{
    // User IDs unique: fresh user_id not in pre.users.
    assert forall|i: int, j: int|
        0 <= i < post.users.len() && 0 <= j < post.users.len() && i != j
        implies #[trigger] post.users[i].user_id != #[trigger] post.users[j].user_id
    by {
        if i == pre.users.len() as int {
            assert(has_user(pre.users, post.users[j].user_id));
        } else if j == pre.users.len() as int {
            assert(has_user(pre.users, post.users[i].user_id));
        }
    }

    // Bills still well-formed: users grew, shares still reference known users.
    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let bill = post.bills[i];
        assert forall|k: int| 0 <= k < bill.payers.len()
            implies has_user(post.users, #[trigger] bill.payers[k].user_id)
        by {
            let w = choose|w: int| 0 <= w < pre.users.len()
                && pre.users[w].user_id == bill.payers[k].user_id;
            assert(post.users[w].user_id == bill.payers[k].user_id);
        }
        assert forall|k: int| 0 <= k < bill.payees.len()
            implies has_user(post.users, #[trigger] bill.payees[k].user_id)
        by {
            let w = choose|w: int| 0 <= w < pre.users.len()
                && pre.users[w].user_id == bill.payees[k].user_id;
            assert(post.users[w].user_id == bill.payees[k].user_id);
        }
        assert forall|j: int| 0 <= j < bill.prev.len()
            implies has_bill(post.bills, #[trigger] bill.prev[j])
        by {
            let w = choose|w: int| 0 <= w < pre.bills.len()
                && pre.bills[w].id == bill.prev[j];
            assert(post.bills[w].id == bill.prev[j]);
        }
    }
}

pub proof fn add_device_preserves(
    pre: LedgerStateSpec, post: LedgerStateSpec, device: DeviceSpec,
)
    requires
        ledger_invariant(pre),
        add_device(pre, post, device),
    ensures
        ledger_invariant(post),
{
    assert forall|i: int, j: int|
        0 <= i < post.devices.len() && 0 <= j < post.devices.len() && i != j
        implies #[trigger] post.devices[i].node_id != #[trigger] post.devices[j].node_id
    by {
        if i == pre.devices.len() as int {
            assert(has_device(pre.devices, post.devices[j].node_id));
        } else if j == pre.devices.len() as int {
            assert(has_device(pre.devices, post.devices[i].node_id));
        }
    }

    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let bill = post.bills[i];
        let w = choose|w: int| 0 <= w < pre.devices.len()
            && pre.devices[w].node_id == bill.created_by_device;
        assert(post.devices[w].node_id == bill.created_by_device);
        assert forall|j: int| 0 <= j < bill.prev.len()
            implies has_bill(post.bills, #[trigger] bill.prev[j])
        by {
            let w = choose|w: int| 0 <= w < pre.bills.len()
                && pre.bills[w].id == bill.prev[j];
            assert(post.bills[w].id == bill.prev[j]);
        }
    }
}

pub proof fn add_bill_preserves(
    pre: LedgerStateSpec, post: LedgerStateSpec, bill: BillSpec,
)
    requires
        ledger_invariant(pre),
        add_bill(pre, post, bill),
    ensures
        ledger_invariant(post),
{
    // Bill IDs unique.
    assert forall|i: int, j: int|
        0 <= i < post.bills.len() && 0 <= j < post.bills.len() && i != j
        implies #[trigger] post.bills[i].id != #[trigger] post.bills[j].id
    by {
        if i == pre.bills.len() as int {
            assert(!has_bill(pre.bills, bill.id));
            assert(has_bill(pre.bills, post.bills[j].id));
        } else if j == pre.bills.len() as int {
            assert(!has_bill(pre.bills, bill.id));
            assert(has_bill(pre.bills, post.bills[i].id));
        }
    }

    // All bills well-formed in post.
    assert forall|i: int| 0 <= i < post.bills.len()
        implies bill_well_formed(
            #[trigger] post.bills[i], post.users, post.devices, post.bills,
        )
    by {
        let b = post.bills[i];
        assert forall|j: int| 0 <= j < b.prev.len()
            implies has_bill(post.bills, #[trigger] b.prev[j])
        by {
            assert(has_bill(pre.bills, b.prev[j]));
            let w = choose|w: int| 0 <= w < pre.bills.len()
                && pre.bills[w].id == b.prev[j];
            assert(post.bills[w].id == b.prev[j]);
        }
        // No self-prev: bill.prev[j] != bill.id.
        assert forall|j: int| 0 <= j < b.prev.len()
            implies #[trigger] b.prev[j] != b.id
        by {
            if i == pre.bills.len() as int {
                // New bill: prev[j] exists in pre.bills, but bill.id is NOT in pre.bills.
                assert(has_bill(pre.bills, b.prev[j]));
                assert(!has_bill(pre.bills, bill.id));
            }
            // Existing bills (i < pre.bills.len()): preserved from ledger_invariant(pre).
        }
    }
}

}
