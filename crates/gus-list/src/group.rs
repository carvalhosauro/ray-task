//! Grouping of consecutive items that share a key.

/// Walks `items` in order and marks where each run of equal group keys starts.
///
/// Yields `(Some(key), item)` for the first item of every run and `(None, item)` for the rest.
/// Only *consecutive* items are grouped: sort by the group key first if you want one run per
/// key. O(n), no reordering.
///
/// ```
/// let days = [(5, "a"), (5, "b"), (6, "c")];
/// let marked: Vec<_> = gus_list::group_runs(&days, |d| d.0).map(|(start, d)| (start, d.1)).collect();
/// assert_eq!(marked, vec![(Some(5), "a"), (None, "b"), (Some(6), "c")]);
/// ```
pub fn group_runs<'a, T: 'a, G: PartialEq + Clone>(
    items: impl IntoIterator<Item = &'a T>,
    group: impl Fn(&T) -> G,
) -> impl Iterator<Item = (Option<G>, &'a T)> {
    let mut last: Option<G> = None;
    items.into_iter().map(move |item| {
        let key = group(item);
        if last.as_ref() == Some(&key) {
            (None, item)
        } else {
            last = Some(key.clone());
            (Some(key), item)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::group_runs;

    fn starts(items: &[(u32, char)]) -> Vec<(Option<u32>, char)> {
        group_runs(items, |item| item.0).map(|(start, item)| (start, item.1)).collect()
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert!(starts(&[]).is_empty());
    }

    #[test]
    fn first_item_of_each_run_carries_the_key() {
        let items = [(1, 'a'), (1, 'b'), (2, 'c'), (3, 'd'), (3, 'e')];
        assert_eq!(starts(&items), vec![(Some(1), 'a'), (None, 'b'), (Some(2), 'c'), (Some(3), 'd'), (None, 'e')]);
    }

    #[test]
    fn single_group_has_one_start() {
        let items = [(7, 'a'), (7, 'b'), (7, 'c')];
        assert_eq!(starts(&items), vec![(Some(7), 'a'), (None, 'b'), (None, 'c')]);
    }

    #[test]
    fn groups_of_one_all_start() {
        let items = [(1, 'a'), (2, 'b'), (3, 'c')];
        assert_eq!(starts(&items), vec![(Some(1), 'a'), (Some(2), 'b'), (Some(3), 'c')]);
    }

    #[test]
    fn a_key_that_comes_back_starts_a_new_run() {
        let items = [(1, 'a'), (2, 'b'), (1, 'c')];
        assert_eq!(starts(&items), vec![(Some(1), 'a'), (Some(2), 'b'), (Some(1), 'c')]);
    }

    #[test]
    fn order_is_preserved() {
        let items = [(2, 'z'), (1, 'y'), (1, 'x')];
        assert_eq!(starts(&items).into_iter().map(|(_, c)| c).collect::<String>(), "zyx");
    }
}
