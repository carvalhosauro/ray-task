//! Slint adapter for gus-list: keeps a `VecModel` in sync with a freshly built list of rows,
//! reusing rows with the same key so their animations keep running.

use std::hash::Hash;

use gus_list::{diff, Op, Plan, DEFAULT_MAX_IN_PLACE};
use slint::{Model, VecModel};

/// Makes `model` hold `rows`, identifying rows by `key`.
///
/// Rows whose key is already in the model are updated in place (only if they changed, by `==`),
/// so per-row animations keep running. When most rows are new, or too many would be inserted or
/// removed, the model is swapped at once with `set_vec`.
pub fn sync<T, K>(model: &VecModel<T>, rows: Vec<T>, key: impl Fn(&T) -> K)
where
    T: Clone + PartialEq + 'static,
    K: Hash + Eq,
{
    let old: Vec<K> = model.iter().map(|row| key(&row)).collect();
    let new: Vec<K> = rows.iter().map(&key).collect();
    let ops = match diff(&old, &new, DEFAULT_MAX_IN_PLACE) {
        Plan::Replace => {
            model.set_vec(rows);
            return;
        }
        Plan::Patch(ops) => ops,
    };
    // diff uses every new index exactly once, so each row is moved out of `rows` once.
    let mut rows: Vec<Option<T>> = rows.into_iter().map(Some).collect();
    let mut take = |from: usize| rows[from].take().expect("diff uses each new row once");
    for op in ops {
        match op {
            Op::Remove { at } => {
                model.remove(at);
            }
            Op::Insert { at, from } => model.insert(at, take(from)),
            Op::Update { at, from } => {
                let row = take(from);
                if model.row_data(at).as_ref() != Some(&row) {
                    model.set_row_data(at, row);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use slint::{Model, VecModel};

    use super::sync;

    /// `stamp` is ignored by `==`: it reveals whether a row was rewritten.
    #[derive(Debug, Clone)]
    struct Row {
        id: u32,
        text: &'static str,
        stamp: u32,
    }

    impl PartialEq for Row {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id && self.text == other.text
        }
    }

    fn row(id: u32, text: &'static str, stamp: u32) -> Row {
        Row { id, text, stamp }
    }

    fn snapshot(model: &VecModel<Row>) -> Vec<(u32, &'static str, u32)> {
        model.iter().map(|r| (r.id, r.text, r.stamp)).collect()
    }

    #[test]
    fn patch_keeps_unchanged_rows_and_rewrites_changed_ones() {
        let model = VecModel::from(vec![row(1, "a", 0), row(2, "b", 0), row(3, "c", 0)]);
        sync(&model, vec![row(1, "a", 1), row(3, "C", 1), row(4, "d", 1)], |r| r.id);
        assert_eq!(snapshot(&model), vec![(1, "a", 0), (3, "C", 1), (4, "d", 1)]);
    }

    #[test]
    fn mostly_new_rows_replace_the_model() {
        let model = VecModel::from(vec![row(1, "a", 0), row(2, "b", 0), row(3, "c", 0)]);
        sync(&model, vec![row(1, "a", 1), row(8, "x", 1), row(9, "y", 1)], |r| r.id);
        assert_eq!(snapshot(&model), vec![(1, "a", 1), (8, "x", 1), (9, "y", 1)], "replace rewrites everything");
    }

    #[test]
    fn rows_that_move_back_are_reinserted() {
        let model = VecModel::from(vec![row(1, "a", 0), row(2, "b", 0), row(3, "c", 0), row(4, "d", 0)]);
        sync(&model, vec![row(1, "a", 1), row(3, "c", 1), row(2, "b", 1), row(4, "d", 1)], |r| r.id);
        assert_eq!(snapshot(&model), vec![(1, "a", 0), (3, "c", 0), (2, "b", 1), (4, "d", 0)]);
    }

    #[test]
    fn sync_to_empty_clears_model() {
        let model = VecModel::from(vec![row(1, "a", 0), row(2, "b", 0)]);
        sync(&model, Vec::new(), |r| r.id);
        assert_eq!(model.row_count(), 0);
    }

    #[test]
    fn sync_from_empty_fills_model() {
        let model = VecModel::<Row>::default();
        sync(&model, vec![row(1, "a", 1), row(2, "b", 1)], |r| r.id);
        assert_eq!(snapshot(&model), vec![(1, "a", 1), (2, "b", 1)]);
    }
}
