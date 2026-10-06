//! Chords bound to app actions.

use std::fmt;

use crate::{Chord, ChordError};

/// Why a binding was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapError {
    Parse(ChordError),
    /// The chord is already bound; the first binding is kept.
    Conflict(Chord),
}

impl fmt::Display for KeymapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeymapError::Parse(error) => write!(f, "{error}"),
            KeymapError::Conflict(chord) => write!(f, "`{chord}` is already bound"),
        }
    }
}

impl std::error::Error for KeymapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KeymapError::Parse(error) => Some(error),
            KeymapError::Conflict(_) => None,
        }
    }
}

impl From<ChordError> for KeymapError {
    fn from(error: ChordError) -> Self {
        KeymapError::Parse(error)
    }
}

/// Chords bound to actions of type `A`, in insertion order.
///
/// ```
/// use gus_keys::Keymap;
/// let mut keys = Keymap::new();
/// keys.bind("Ctrl+N", "new task").unwrap();
/// assert_eq!(keys.lookup(&"ctrl+n".parse().unwrap()), Some(&"new task"));
/// assert_eq!(keys.lookup(&"Ctrl+Shift+N".parse().unwrap()), None);
/// ```
pub struct Keymap<A> {
    bindings: Vec<(Chord, A)>,
}

impl<A> Default for Keymap<A> {
    fn default() -> Self {
        Self { bindings: Vec::new() }
    }
}

impl<A> Keymap<A> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `chord` (e.g. `"Ctrl+Shift+N"`) to `action`. Fails on a bad chord or when the chord
    /// is already bound.
    pub fn bind(&mut self, chord: &str, action: A) -> Result<(), KeymapError> {
        let chord: Chord = chord.parse()?;
        if self.bindings.iter().any(|(bound, _)| *bound == chord) {
            return Err(KeymapError::Conflict(chord));
        }
        self.bindings.push((chord, action));
        Ok(())
    }

    /// The action bound to exactly this chord.
    pub fn lookup(&self, chord: &Chord) -> Option<&A> {
        self.bindings.iter().find(|(bound, _)| bound == chord).map(|(_, action)| action)
    }

    /// Every bound chord, in the order they were bound.
    pub fn chords(&self) -> impl Iterator<Item = &Chord> + '_ {
        self.bindings.iter().map(|(chord, _)| chord)
    }
}

#[cfg(test)]
mod tests {
    use super::{Keymap, KeymapError};
    use crate::{Chord, ChordError};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Action {
        New,
        NewProject,
        Undo,
    }

    fn chord(s: &str) -> Chord {
        s.parse().unwrap()
    }

    fn map() -> Keymap<Action> {
        let mut map = Keymap::new();
        map.bind("Ctrl+N", Action::New).unwrap();
        map.bind("Ctrl+Shift+N", Action::NewProject).unwrap();
        map.bind("Ctrl+Z", Action::Undo).unwrap();
        map
    }

    #[test]
    fn lookup_finds_the_bound_action() {
        let map = map();
        assert_eq!(map.lookup(&chord("Ctrl+N")), Some(&Action::New));
        assert_eq!(map.lookup(&chord("Ctrl+Shift+N")), Some(&Action::NewProject));
        assert_eq!(map.lookup(&chord("ctrl+z")), Some(&Action::Undo));
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let map = map();
        assert_eq!(map.lookup(&chord("Ctrl+Shift+Z")), None);
        assert_eq!(map.lookup(&chord("Z")), None);
        assert_eq!(map.lookup(&chord("Ctrl+Alt+N")), None);
    }

    #[test]
    fn unknown_chord_is_none() {
        assert_eq!(map().lookup(&chord("Ctrl+Q")), None);
        assert_eq!(Keymap::<Action>::new().lookup(&chord("Ctrl+N")), None);
    }

    #[test]
    fn binding_the_same_chord_twice_is_a_conflict() {
        let mut map = map();
        assert_eq!(map.bind("ctrl + n", Action::Undo), Err(KeymapError::Conflict(chord("Ctrl+N"))));
        assert_eq!(map.lookup(&chord("Ctrl+N")), Some(&Action::New), "the first binding stays");
    }

    #[test]
    fn bad_chord_is_a_parse_error() {
        let mut map = map();
        assert_eq!(map.bind("Hyper+N", Action::New), Err(KeymapError::Parse(ChordError::UnknownModifier("Hyper".into()))));
    }

    #[test]
    fn chords_keep_insertion_order() {
        let names: Vec<String> = map().chords().map(ToString::to_string).collect();
        assert_eq!(names, ["Ctrl+N", "Ctrl+Shift+N", "Ctrl+Z"]);
    }

    #[test]
    fn errors_have_readable_messages() {
        assert_eq!(KeymapError::Conflict(chord("Ctrl+N")).to_string(), "`Ctrl+N` is already bound");
        assert_eq!(KeymapError::Parse(ChordError::Empty).to_string(), "empty chord");
        assert_eq!(ChordError::UnknownKey("Foo".into()).to_string(), "unknown key `Foo`");
        assert_eq!(ChordError::UnknownModifier("Hyper".into()).to_string(), "unknown modifier `Hyper`");
    }
}
