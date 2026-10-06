//! Moving a keyboard selection through a list by key.

/// Returns the key `delta` positions away from `current`, counting only `active` keys.
///
/// - No active keys: `None`.
/// - `current` is `None`, inactive or not in `keys`: `delta >= 0` picks the first active key,
///   `delta < 0` the last.
/// - Otherwise the move is clamped to the ends (no wrap-around). Arithmetic saturates, so
///   `isize::MIN` / `isize::MAX` work as Home / End.
///
/// ```
/// let keys = [1, 2, 3];
/// assert_eq!(gus_list::step(&keys, Some(&1), 1, |_| true), Some(&2));
/// assert_eq!(gus_list::step(&keys, Some(&1), 1, |k| *k != 2), Some(&3));
/// assert_eq!(gus_list::step(&keys, Some(&3), 1, |_| true), Some(&3));
/// ```
pub fn step<'a, K: PartialEq>(keys: &'a [K], current: Option<&K>, delta: isize, active: impl Fn(&K) -> bool) -> Option<&'a K> {
    let live: Vec<&'a K> = keys.iter().filter(|k| active(k)).collect();
    let last = live.len().checked_sub(1)?;
    let next = match current.and_then(|c| live.iter().position(|k| *k == c)) {
        Some(i) => i.saturating_add_signed(delta).min(last),
        None if delta >= 0 => 0,
        None => last,
    };
    Some(live[next])
}

#[cfg(test)]
mod tests {
    use super::step;

    const KEYS: [u32; 4] = [10, 20, 30, 40];

    fn all(_: &u32) -> bool {
        true
    }

    #[test]
    fn empty_list_selects_nothing() {
        assert_eq!(step::<u32>(&[], None, 1, all), None);
        assert_eq!(step::<u32>(&[], Some(&10), -1, all), None);
    }

    #[test]
    fn all_inactive_selects_nothing() {
        assert_eq!(step(&KEYS, Some(&20), 1, |_| false), None);
    }

    #[test]
    fn no_selection_starts_at_first_going_down_and_last_going_up() {
        assert_eq!(step(&KEYS, None, 1, all), Some(&10));
        assert_eq!(step(&KEYS, None, 0, all), Some(&10));
        assert_eq!(step(&KEYS, None, -1, all), Some(&40));
    }

    #[test]
    fn moves_by_delta_and_clamps_without_wrapping() {
        assert_eq!(step(&KEYS, Some(&20), 1, all), Some(&30));
        assert_eq!(step(&KEYS, Some(&20), -1, all), Some(&10));
        assert_eq!(step(&KEYS, Some(&40), 1, all), Some(&40));
        assert_eq!(step(&KEYS, Some(&10), -5, all), Some(&10));
        assert_eq!(step(&KEYS, Some(&10), 2, all), Some(&30));
    }

    #[test]
    fn inactive_keys_are_skipped() {
        let active = |k: &u32| *k != 20 && *k != 30;
        assert_eq!(step(&KEYS, Some(&10), 1, active), Some(&40));
        assert_eq!(step(&KEYS, Some(&40), -1, active), Some(&10));
    }

    #[test]
    fn inactive_or_unknown_current_counts_as_no_selection() {
        let active = |k: &u32| *k != 30;
        assert_eq!(step(&KEYS, Some(&30), 1, active), Some(&10));
        assert_eq!(step(&KEYS, Some(&30), -1, active), Some(&40));
        assert_eq!(step(&KEYS, Some(&99), 1, all), Some(&10));
    }

    #[test]
    fn step_saturates_extreme_deltas() {
        assert_eq!(step(&KEYS, Some(&20), isize::MAX, all), Some(&40));
        assert_eq!(step(&KEYS, Some(&30), isize::MIN, all), Some(&10));
        assert_eq!(step(&KEYS, None, isize::MIN, all), Some(&40));
    }
}
