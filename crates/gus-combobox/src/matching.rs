//! Which options match the typed text, and which one completes it inline.

/// Options matching `query`, case-insensitive, with the query trimmed: first those whose label
/// starts with the query, then those that only contain it; input order is kept within each
/// group. An empty query matches everything.
///
/// ```
/// let tags = ["casa", "ui-kit", "kit"];
/// assert_eq!(gus_combobox::matches(&tags, "KIT", |t| *t), [&"kit", &"ui-kit"]);
/// ```
pub fn matches<'a, T>(options: impl IntoIterator<Item = &'a T>, query: &str, label: impl Fn(&T) -> &str) -> Vec<&'a T>
where
    T: 'a,
{
    let query = query.trim().to_lowercase();
    let (mut starts, mut contains) = (Vec::new(), Vec::new());
    for option in options {
        let name = label(option).to_lowercase();
        if name.starts_with(&query) {
            starts.push(option);
        } else if name.contains(&query) {
            contains.push(option);
        }
    }
    starts.extend(contains);
    starts
}

/// The inline completion: index into `matches` of the first one whose label starts with the
/// query (never one that only contains it). `None` for an empty query.
pub fn completion<T>(matches: &[&T], query: &str, label: impl Fn(&T) -> &str) -> Option<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return None;
    }
    matches.iter().position(|option| label(option).to_lowercase().starts_with(&query))
}

#[cfg(test)]
mod tests {
    use super::{completion, matches};

    const TAGS: [&str; 5] = ["carro", "casa", "ui-kit", "Cama", "kit"];

    fn names(query: &str) -> Vec<&'static str> {
        matches(&TAGS, query, |t| *t).into_iter().copied().collect()
    }

    #[test]
    fn empty_query_returns_everything_in_order() {
        assert_eq!(names(""), TAGS);
        assert_eq!(names("   "), TAGS);
    }

    #[test]
    fn starts_with_comes_before_contains() {
        assert_eq!(names("kit"), ["kit", "ui-kit"]);
    }

    #[test]
    fn order_is_stable_within_each_group() {
        assert_eq!(names("ca"), ["carro", "casa", "Cama"]);
        assert_eq!(names("a"), ["carro", "casa", "Cama"], "contains-only matches keep input order");
    }

    #[test]
    fn case_and_surrounding_spaces_are_ignored() {
        assert_eq!(names("  CA "), ["carro", "casa", "Cama"]);
        assert_eq!(names("ui-KIT"), ["ui-kit"]);
    }

    #[test]
    fn no_match_is_empty() {
        assert!(names("zzz").is_empty());
    }

    #[test]
    fn completion_is_the_first_starts_with_match() {
        let found = matches(&TAGS, "ca", |t| *t);
        assert_eq!(completion(&found, "ca", |t| *t), Some(0));
        let found = matches(&TAGS, "Ki", |t| *t);
        assert_eq!(completion(&found, "Ki", |t| *t), Some(0));
    }

    #[test]
    fn completion_never_picks_a_contains_only_match() {
        let found = matches(&TAGS, "-kit", |t| *t);
        assert_eq!(found.len(), 1);
        assert_eq!(completion(&found, "-kit", |t| *t), None);
    }

    #[test]
    fn completion_of_empty_query_is_none() {
        let found = matches(&TAGS, "", |t| *t);
        assert_eq!(completion(&found, "", |t| *t), None);
        assert_eq!(completion(&found, "  ", |t| *t), None);
    }
}
