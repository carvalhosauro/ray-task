//! Keyed diff between two lists of keys, as a plan a retained-mode model can apply.

use std::collections::HashMap;
use std::hash::Hash;

/// Above this many insertions + removals, [`diff`] callers usually prefer swapping the whole
/// model: each in-place insert/remove is O(n) and there is no animation worth keeping.
pub const DEFAULT_MAX_IN_PLACE: usize = 64;

/// What to do to turn the old list into the new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Swap the whole model for the new rows.
    Replace,
    /// Apply these operations, in order, to a model that currently holds the old rows.
    Patch(Vec<Op>),
}

/// One model operation. `at` is valid on the model *at the moment the op runs*; `from` is an
/// index into the new list. Every new index appears exactly once as a `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Remove the row at `at`.
    Remove { at: usize },
    /// Insert `new[from]` at `at`.
    Insert { at: usize, from: usize },
    /// The row at `at` has the same key as `new[from]`: refresh its data in place.
    Update { at: usize, from: usize },
}

/// Plans how to turn a model holding rows keyed `old` into one keyed `new`, reusing rows with the
/// same key so their animations keep running.
///
/// Returns [`Plan::Replace`] when fewer than half the rows are reused or when insertions plus
/// removals exceed `max_in_place`. Keys must be unique within each list. A row that moved
/// backwards is removed and re-inserted (there is no move op).
///
/// ```
/// use gus_list::{diff, Op, Plan, DEFAULT_MAX_IN_PLACE};
/// let plan = diff(&[1, 2, 3], &[1, 3], DEFAULT_MAX_IN_PLACE);
/// assert_eq!(plan, Plan::Patch(vec![Op::Update { at: 0, from: 0 }, Op::Remove { at: 1 }, Op::Update { at: 1, from: 1 }]));
/// ```
pub fn diff<K: Hash + Eq>(old: &[K], new: &[K], max_in_place: usize) -> Plan {
    let old_at = positions(old);
    debug_assert_eq!(old_at.len(), old.len(), "diff: duplicate keys in old");
    debug_assert_eq!(positions(new).len(), new.len(), "diff: duplicate keys in new");
    let reused = new.iter().filter(|k| old_at.contains_key(k)).count();
    let changes = (new.len() - reused) + (old.len() - reused);
    if reused * 2 < new.len().max(old.len()) || changes > max_in_place {
        return Plan::Replace;
    }
    Plan::Patch(patch_with(&old_at, old.len(), new))
}

/// The in-place operations, without the replace heuristic.
///
/// Invariant during the pass: the model is `new[..i]` followed by `old[cursor..]`, so an old
/// position tells how many rows to drop before reusing it.
#[cfg(test)]
pub(crate) fn patch<K: Hash + Eq>(old: &[K], new: &[K]) -> Vec<Op> {
    patch_with(&positions(old), old.len(), new)
}

fn patch_with<K: Hash + Eq>(positions: &HashMap<&K, usize>, old_len: usize, new: &[K]) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut cursor = 0;
    for (i, key) in new.iter().enumerate() {
        match positions.get(key).copied().filter(|&j| j >= cursor) {
            Some(j) => {
                ops.extend((cursor..j).map(|_| Op::Remove { at: i }));
                cursor = j + 1;
                ops.push(Op::Update { at: i, from: i });
            }
            None => ops.push(Op::Insert { at: i, from: i }),
        }
    }
    let leftover = old_len - cursor;
    ops.extend((new.len()..new.len() + leftover).rev().map(|at| Op::Remove { at }));
    ops
}

fn positions<K: Hash + Eq>(keys: &[K]) -> HashMap<&K, usize> {
    keys.iter().enumerate().map(|(i, k)| (k, i)).collect()
}

#[cfg(test)]
mod tests {
    use super::{diff, patch, Op, Plan, DEFAULT_MAX_IN_PLACE};

    /// Every ordered sequence of distinct keys drawn from `0..n` (including the empty one).
    fn sequences(n: u8) -> Vec<Vec<u8>> {
        let mut out = vec![Vec::new()];
        let mut frontier: Vec<Vec<u8>> = vec![Vec::new()];
        for _ in 0..n {
            let mut next = Vec::new();
            for seq in &frontier {
                for k in (0..n).filter(|k| !seq.contains(k)) {
                    let mut grown = seq.clone();
                    grown.push(k);
                    next.push(grown);
                }
            }
            out.extend(next.iter().cloned());
            frontier = next;
        }
        out
    }

    /// Applies `ops` to a simulated model that starts as `old`.
    fn apply(old: &[u8], new: &[u8], ops: &[Op]) -> Vec<u8> {
        let mut model = old.to_vec();
        for op in ops {
            match *op {
                Op::Remove { at } => {
                    model.remove(at);
                }
                Op::Insert { at, from } => model.insert(at, new[from]),
                Op::Update { at, from } => assert_eq!(model[at], new[from], "update must hit the row with the same key"),
            }
        }
        model
    }

    fn froms(ops: &[Op]) -> Vec<usize> {
        let mut out: Vec<usize> = ops
            .iter()
            .filter_map(|op| match *op {
                Op::Insert { from, .. } | Op::Update { from, .. } => Some(from),
                Op::Remove { .. } => None,
            })
            .collect();
        out.sort_unstable();
        out
    }

    #[test]
    fn patch_turns_every_old_into_every_new() {
        let all = sequences(5);
        assert_eq!(all.len(), 326);
        for old in &all {
            for new in &all {
                let ops = patch(old, new);
                assert_eq!(apply(old, new, &ops), *new, "old={old:?} new={new:?} ops={ops:?}");
                assert_eq!(froms(&ops), (0..new.len()).collect::<Vec<_>>(), "each new row used once: old={old:?} new={new:?}");
            }
        }
    }

    #[test]
    fn identical_lists_only_update() {
        let keys = [1, 2, 3];
        let expected = Plan::Patch(vec![Op::Update { at: 0, from: 0 }, Op::Update { at: 1, from: 1 }, Op::Update { at: 2, from: 2 }]);
        assert_eq!(diff(&keys, &keys, DEFAULT_MAX_IN_PLACE), expected);
    }

    #[test]
    fn both_empty_is_an_empty_patch() {
        assert_eq!(diff::<u8>(&[], &[], DEFAULT_MAX_IN_PLACE), Plan::Patch(Vec::new()));
    }

    #[test]
    fn filling_an_empty_list_replaces() {
        assert_eq!(diff(&[], &[1, 2, 3], DEFAULT_MAX_IN_PLACE), Plan::Replace);
    }

    #[test]
    fn half_reused_still_patches() {
        assert!(matches!(diff(&[1, 2], &[1, 3], DEFAULT_MAX_IN_PLACE), Plan::Patch(_)));
    }

    #[test]
    fn less_than_half_reused_replaces() {
        assert_eq!(diff(&[1, 2, 3], &[1, 4, 5], DEFAULT_MAX_IN_PLACE), Plan::Replace);
    }

    #[test]
    fn too_many_changes_replace() {
        let old: Vec<u32> = (0..10).collect();
        let new: Vec<u32> = (0..20).collect();
        assert_eq!(diff(&old, &new, 5), Plan::Replace);
        assert!(matches!(diff(&old, &new, DEFAULT_MAX_IN_PLACE), Plan::Patch(_)));
    }

    #[test]
    fn trailing_rows_are_removed_from_the_end() {
        let Plan::Patch(ops) = diff(&[1, 2, 3, 4], &[1, 2, 3], DEFAULT_MAX_IN_PLACE) else { panic!("expected a patch") };
        assert_eq!(ops.last(), Some(&Op::Remove { at: 3 }));
    }
}
