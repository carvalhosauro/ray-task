//! The list's state: open or closed, and which match (if any) is highlighted.

/// A key the combobox understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Down,
    Up,
    Enter,
    Escape,
    Tab,
}

/// What the app should do after a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Not for the combobox: let the event through.
    Ignored,
    /// The highlight moved (or the list closed via Up past the first item).
    Moved,
    /// Use match `i` (the list closed).
    Pick(usize),
    /// Nothing was highlighted: use the typed text (the list closed).
    UseText,
    /// Replace the text with match `i` (the inline completion).
    Complete(usize),
    /// The list closed.
    Closed,
}

/// Open/closed and the highlighted match. The app keeps the text and the matches and passes
/// the current number of matches to [`Combobox::key`].
///
/// ```
/// use gus_combobox::{Combobox, Nav, Outcome};
/// let mut list = Combobox::default();
/// list.open();
/// assert_eq!(list.key(Nav::Enter, 3, None), Outcome::UseText);
/// list.input();
/// list.key(Nav::Down, 3, None);
/// assert_eq!(list.key(Nav::Enter, 3, None), Outcome::Pick(0));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Combobox {
    open: bool,
    highlighted: Option<usize>,
}

impl Combobox {
    /// The field got focus: open, nothing highlighted.
    pub fn open(&mut self) {
        self.open = true;
        self.highlighted = None;
    }

    /// The field lost focus: closed, nothing highlighted.
    pub fn close(&mut self) {
        self.open = false;
        self.highlighted = None;
    }

    /// The text changed: open, nothing highlighted.
    pub fn input(&mut self) {
        self.open();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    /// Applies a key, given how many matches are shown and the inline completion (if any).
    pub fn key(&mut self, nav: Nav, matches: usize, completion: Option<usize>) -> Outcome {
        match nav {
            Nav::Down => {
                if matches == 0 {
                    return Outcome::Ignored;
                }
                self.open = true;
                self.highlighted = Some(self.highlighted.map_or(0, |i| (i + 1).min(matches - 1)));
                Outcome::Moved
            }
            Nav::Up => match self.highlighted {
                None => Outcome::Ignored,
                Some(0) => {
                    self.highlighted = None;
                    Outcome::Moved
                }
                Some(_) if matches == 0 => {
                    self.highlighted = None;
                    Outcome::Moved
                }
                Some(i) => {
                    self.highlighted = Some((i - 1).min(matches - 1));
                    Outcome::Moved
                }
            },
            Nav::Enter => {
                let outcome = match self.highlighted {
                    Some(i) if i < matches => Outcome::Pick(i),
                    _ => Outcome::UseText,
                };
                self.close();
                outcome
            }
            Nav::Tab => match completion {
                Some(i) => {
                    self.highlighted = None;
                    Outcome::Complete(i)
                }
                None => Outcome::Ignored,
            },
            // Only close what is visible: open but with no matches, Esc goes through to the app.
            Nav::Escape if self.open && matches > 0 => {
                self.close();
                Outcome::Closed
            }
            Nav::Escape => Outcome::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Combobox, Nav, Outcome};

    fn highlighted_at(i: usize, matches: usize) -> Combobox {
        let mut c = Combobox::default();
        c.open();
        for _ in 0..=i {
            c.key(Nav::Down, matches, None);
        }
        assert_eq!(c.highlighted(), Some(i));
        c
    }

    #[test]
    fn down_from_nothing_highlights_first() {
        let mut c = Combobox::default();
        assert_eq!(c.key(Nav::Down, 3, None), Outcome::Moved);
        assert_eq!(c.highlighted(), Some(0));
        assert!(c.is_open(), "Down reopens a closed list");
    }

    #[test]
    fn down_clamps_at_last() {
        let mut c = highlighted_at(2, 3);
        assert_eq!(c.key(Nav::Down, 3, None), Outcome::Moved);
        assert_eq!(c.highlighted(), Some(2));
    }

    #[test]
    fn down_without_matches_is_ignored() {
        let mut c = Combobox::default();
        c.open();
        assert_eq!(c.key(Nav::Down, 0, None), Outcome::Ignored);
        assert_eq!(c.highlighted(), None);
    }

    #[test]
    fn up_from_first_returns_to_text() {
        let mut c = highlighted_at(0, 3);
        assert_eq!(c.key(Nav::Up, 3, None), Outcome::Moved);
        assert_eq!(c.highlighted(), None);
        assert!(c.is_open());
    }

    #[test]
    fn up_without_highlight_is_ignored() {
        let mut c = Combobox::default();
        c.open();
        assert_eq!(c.key(Nav::Up, 3, None), Outcome::Ignored);
    }

    #[test]
    fn up_moves_and_clamps_after_list_shrank() {
        let mut c = highlighted_at(2, 3);
        assert_eq!(c.key(Nav::Up, 3, None), Outcome::Moved);
        assert_eq!(c.highlighted(), Some(1));
        let mut c = highlighted_at(4, 5);
        assert_eq!(c.key(Nav::Up, 2, None), Outcome::Moved);
        assert_eq!(c.highlighted(), Some(1), "clamped to the last of 2");
        let mut c = highlighted_at(4, 5);
        assert_eq!(c.key(Nav::Up, 0, None), Outcome::Moved);
        assert_eq!(c.highlighted(), None);
    }

    #[test]
    fn enter_picks_highlighted_and_closes() {
        let mut c = highlighted_at(1, 3);
        assert_eq!(c.key(Nav::Enter, 3, None), Outcome::Pick(1));
        assert!(!c.is_open());
        assert_eq!(c.highlighted(), None);
    }

    #[test]
    fn enter_without_highlight_uses_text() {
        let mut c = Combobox::default();
        c.open();
        assert_eq!(c.key(Nav::Enter, 3, Some(0)), Outcome::UseText);
        assert!(!c.is_open());
    }

    #[test]
    fn enter_with_stale_highlight_uses_text() {
        let mut c = highlighted_at(4, 5);
        assert_eq!(c.key(Nav::Enter, 2, None), Outcome::UseText);
    }

    #[test]
    fn tab_completes_when_there_is_a_completion() {
        let mut c = highlighted_at(1, 3);
        assert_eq!(c.key(Nav::Tab, 3, Some(0)), Outcome::Complete(0));
        assert_eq!(c.highlighted(), None);
        assert!(c.is_open());
    }

    #[test]
    fn tab_without_completion_is_ignored() {
        let mut c = Combobox::default();
        c.open();
        assert_eq!(c.key(Nav::Tab, 3, None), Outcome::Ignored);
    }

    #[test]
    fn escape_closes_a_visible_list() {
        let mut c = highlighted_at(0, 3);
        assert_eq!(c.key(Nav::Escape, 3, None), Outcome::Closed);
        assert!(!c.is_open());
        assert_eq!(c.highlighted(), None);
    }

    #[test]
    fn escape_without_visible_list_is_ignored() {
        let mut c = Combobox::default();
        c.open();
        assert_eq!(c.key(Nav::Escape, 0, None), Outcome::Ignored, "open but empty: nothing to close");
    }

    #[test]
    fn escape_when_closed_is_ignored() {
        let mut c = Combobox::default();
        assert_eq!(c.key(Nav::Escape, 3, None), Outcome::Ignored);
    }

    #[test]
    fn open_close_input_reset_the_highlight() {
        let mut c = highlighted_at(1, 3);
        c.close();
        assert!(!c.is_open());
        assert_eq!(c.highlighted(), None);
        let mut c = highlighted_at(1, 3);
        c.open();
        assert!(c.is_open());
        assert_eq!(c.highlighted(), None);
    }

    #[test]
    fn input_reopens_and_clears_highlight() {
        let mut c = highlighted_at(1, 3);
        c.key(Nav::Enter, 3, None);
        assert!(!c.is_open());
        c.input();
        assert!(c.is_open());
        assert_eq!(c.highlighted(), None);
        assert_eq!(c.key(Nav::Enter, 3, None), Outcome::UseText);
    }
}
