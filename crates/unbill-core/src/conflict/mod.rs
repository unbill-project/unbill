// Conflict detection: finds effective bills that share amendment ancestry.
// See DESIGN.md for the Union-Find algorithm and ConflictGroup semantics.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::model::{Bill, Ulid};

/// A set of effective bills that conflict, together with their shared amendment history.
///
/// A conflict exists when two or more effective bills are in the same Union-Find component
/// after all `prev` links have been processed. None of the conflicting bills supersedes
/// the others, so the group must be resolved manually by creating a new amendment whose
/// `prev` includes every bill in `conflicting`.
#[derive(Clone, Debug)]
pub struct ConflictGroup {
    /// Effective bills in the component. Always at least two members.
    pub conflicting: Vec<Bill>,
    /// Non-effective bills in the same component — the superseded history that led to the conflict.
    pub ancestors: Vec<Bill>,
}

struct UnionFind {
    parent: HashMap<Ulid, Ulid>,
    rank: HashMap<Ulid, usize>,
}

impl UnionFind {
    fn new() -> Self {
        Self {
            parent: HashMap::new(),
            rank: HashMap::new(),
        }
    }

    fn insert(&mut self, x: Ulid) {
        self.parent.entry(x).or_insert(x);
        self.rank.entry(x).or_insert(0);
    }

    fn find(&mut self, x: Ulid) -> Ulid {
        let parent = *self.parent.get(&x).unwrap_or(&x);
        if parent == x {
            return x;
        }
        let root = self.find(parent);
        self.parent.insert(x, root);
        root
    }

    fn union(&mut self, x: Ulid, y: Ulid) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return;
        }
        let rank_x = *self.rank.get(&rx).unwrap_or(&0);
        let rank_y = *self.rank.get(&ry).unwrap_or(&0);
        match rank_x.cmp(&rank_y) {
            std::cmp::Ordering::Less => {
                self.parent.insert(rx, ry);
            }
            std::cmp::Ordering::Greater => {
                self.parent.insert(ry, rx);
            }
            std::cmp::Ordering::Equal => {
                self.parent.insert(ry, rx);
                *self.rank.entry(rx).or_default() += 1;
            }
        }
    }
}

/// Detect amendment conflicts in a full bill list.
///
/// Returns one `ConflictGroup` per set of effective bills that share a Union-Find
/// component. Bills with no conflicting siblings are not included.
pub fn detect(bills: &[Bill]) -> Vec<ConflictGroup> {
    let mut uf = UnionFind::new();

    for bill in bills {
        uf.insert(bill.id);
        for &prev_id in &bill.prev {
            uf.insert(prev_id);
        }
    }

    for bill in bills {
        for &prev_id in &bill.prev {
            uf.union(bill.id, prev_id);
        }
    }

    let superseded: HashSet<Ulid> = bills.iter().flat_map(|b| b.prev.iter().copied()).collect();

    // Group bills by root into (conflicting, ancestors).
    let mut groups: HashMap<Ulid, (Vec<Bill>, Vec<Bill>)> = HashMap::new();
    for bill in bills {
        let root = uf.find(bill.id);
        let (conflicting, ancestors) = groups.entry(root).or_default();
        if superseded.contains(&bill.id) {
            ancestors.push(bill.clone());
        } else {
            conflicting.push(bill.clone());
        }
    }

    groups
        .into_values()
        .filter(|(conflicting, _)| conflicting.len() >= 2)
        .map(|(conflicting, ancestors)| ConflictGroup {
            conflicting,
            ancestors,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeId, Share, Timestamp};

    fn device() -> NodeId {
        NodeId::from_seed(1)
    }

    fn uid(n: u128) -> Ulid {
        Ulid::from_u128(n)
    }

    fn bill(id: u128, prev: &[u128]) -> Bill {
        Bill {
            id: uid(id),
            amount_cents: 100,
            description: String::new(),
            payers: vec![Share {
                user_id: uid(99),
                shares: 1,
            }],
            payees: vec![Share {
                user_id: uid(99),
                shares: 1,
            }],
            prev: prev.iter().map(|&p| uid(p)).collect(),
            created_at: Timestamp::from_millis(0),
            created_by_device: device(),
        }
    }

    #[test]
    fn test_empty_bill_list_produces_no_conflicts() {
        assert!(detect(&[]).is_empty());
    }

    #[test]
    fn test_no_amendments_produces_no_conflicts() {
        let bills = vec![bill(1, &[]), bill(2, &[]), bill(3, &[])];
        assert!(detect(&bills).is_empty());
    }

    #[test]
    fn test_linear_amendment_chain_produces_no_conflicts() {
        // 1 <- 2 <- 3
        let bills = vec![bill(1, &[]), bill(2, &[1]), bill(3, &[2])];
        assert!(detect(&bills).is_empty());
    }

    #[test]
    fn test_two_independent_amendments_produce_one_conflict() {
        // 2 and 3 both amend 1
        let bills = vec![bill(1, &[]), bill(2, &[1]), bill(3, &[1])];
        let groups = detect(&bills);
        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(group.conflicting.len(), 2);
        assert_eq!(group.ancestors.len(), 1);
        assert_eq!(group.ancestors[0].id, uid(1));
    }

    #[test]
    fn test_merging_amendment_removes_conflict() {
        // 2 and 3 both amend 1; 4 merges 2 and 3
        let bills = vec![bill(1, &[]), bill(2, &[1]), bill(3, &[1]), bill(4, &[2, 3])];
        assert!(detect(&bills).is_empty());
    }

    #[test]
    fn test_nested_fork_conflict() {
        // 1 <- 2 <- 4 (peer 1)
        //      2 <- 5 (peer 2 independently amends 2)
        // 4 and 5 conflict; ancestors are 1 and 2
        let bills = vec![bill(1, &[]), bill(2, &[1]), bill(4, &[2]), bill(5, &[2])];
        let groups = detect(&bills);
        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(group.conflicting.len(), 2);
        assert_eq!(group.ancestors.len(), 2);
    }

    #[test]
    fn test_ancestors_are_disjoint_from_conflicting() {
        let bills = vec![bill(1, &[]), bill(2, &[1]), bill(3, &[1])];
        let groups = detect(&bills);
        let group = &groups[0];
        let conflicting_ids: HashSet<Ulid> = group.conflicting.iter().map(|b| b.id).collect();
        for ancestor in &group.ancestors {
            assert!(!conflicting_ids.contains(&ancestor.id));
        }
    }

    #[test]
    fn test_independent_bills_outside_conflict_are_excluded() {
        // Bills 4 and 5 are unrelated originals; only 2 and 3 conflict
        let bills = vec![
            bill(1, &[]),
            bill(2, &[1]),
            bill(3, &[1]),
            bill(4, &[]),
            bill(5, &[]),
        ];
        let groups = detect(&bills);
        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        let all_ids: HashSet<Ulid> = group
            .conflicting
            .iter()
            .chain(group.ancestors.iter())
            .map(|b| b.id)
            .collect();
        assert!(!all_ids.contains(&uid(4)));
        assert!(!all_ids.contains(&uid(5)));
    }

    #[test]
    fn test_results_are_deterministic_regardless_of_insertion_order() {
        let bills_a = vec![bill(1, &[]), bill(2, &[1]), bill(3, &[1])];
        let bills_b = vec![bill(3, &[1]), bill(1, &[]), bill(2, &[1])];
        let groups_a = detect(&bills_a);
        let groups_b = detect(&bills_b);
        assert_eq!(groups_a.len(), groups_b.len());
        assert_eq!(groups_a[0].conflicting.len(), groups_b[0].conflicting.len());
        assert_eq!(groups_a[0].ancestors.len(), groups_b[0].ancestors.len());
    }
}
