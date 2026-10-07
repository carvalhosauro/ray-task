//! Key chords: a key plus modifiers, parsed from and printed as `Ctrl+Shift+N`.

use std::fmt;
use std::str::FromStr;

/// A key, independent of any UI toolkit. Letters and other printable keys are [`Key::Char`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Delete,
    Backspace,
    Tab,
    Home,
    End,
    PageUp,
    PageDown,
    /// Function keys `F1` to `F12`.
    F(u8),
}

/// Modifier keys held with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

/// A key plus modifiers. Matching is exact: `Ctrl+Shift+Z` is not `Ctrl+Z`.
///
/// ```
/// use gus_keys::{Chord, Key, Mods};
/// let chord: Chord = "Ctrl+Shift+N".parse().unwrap();
/// assert_eq!(chord, Chord::new(Key::Char('N'), Mods { ctrl: true, shift: true, ..Mods::default() }));
/// assert_eq!(chord.to_string(), "Ctrl+Shift+N");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub key: Key,
    pub mods: Mods,
}

impl Chord {
    /// Builds a chord. Letters are stored lowercase, so Caps Lock or a platform that reports
    /// `N` without Shift still matches `Ctrl+N`.
    pub fn new(key: Key, mods: Mods) -> Self {
        let key = match key {
            Key::Char(c) => Key::Char(lowercase(c)),
            other => other,
        };
        Self { key, mods }
    }
}

fn lowercase(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(single), None) => single,
        _ => c,
    }
}

/// Why a chord string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChordError {
    /// Nothing where the key should be (`""`, `"Ctrl+"`).
    Empty,
    /// Not a key name and not a single character.
    UnknownKey(String),
    /// Not one of `Ctrl`, `Control`, `Shift`, `Alt`, `Meta`, `Super`, `Cmd`.
    UnknownModifier(String),
}

impl fmt::Display for ChordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChordError::Empty => f.write_str("empty chord"),
            ChordError::UnknownKey(key) => write!(f, "unknown key `{key}`"),
            ChordError::UnknownModifier(modifier) => write!(f, "unknown modifier `{modifier}`"),
        }
    }
}

impl std::error::Error for ChordError {}

impl FromStr for Chord {
    type Err = ChordError;

    /// `+`-separated, modifiers first, key last; case-insensitive; spaces around parts ignored.
    fn from_str(s: &str) -> Result<Self, ChordError> {
        let parts: Vec<&str> = s.split('+').map(str::trim).collect();
        let Some((key, modifiers)) = parts.split_last() else { return Err(ChordError::Empty) };
        let mut mods = Mods::default();
        for modifier in modifiers {
            match modifier.to_lowercase().as_str() {
                "ctrl" | "control" => mods.ctrl = true,
                "shift" => mods.shift = true,
                "alt" => mods.alt = true,
                "meta" | "super" | "cmd" => mods.meta = true,
                _ => return Err(ChordError::UnknownModifier((*modifier).to_string())),
            }
        }
        Ok(Chord::new(parse_key(key)?, mods))
    }
}

fn parse_key(name: &str) -> Result<Key, ChordError> {
    if name.is_empty() {
        return Err(ChordError::Empty);
    }
    let lower = name.to_lowercase();
    if let Some(n) = lower.strip_prefix('f').filter(|d| !d.is_empty()).and_then(|d| d.parse::<u8>().ok()) {
        return if (1..=12).contains(&n) { Ok(Key::F(n)) } else { Err(ChordError::UnknownKey(name.to_string())) };
    }
    let key = match lower.as_str() {
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "enter" | "return" => Key::Enter,
        "esc" | "escape" => Key::Escape,
        "delete" | "del" => Key::Delete,
        "backspace" => Key::Backspace,
        "tab" => Key::Tab,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        _ => {
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Char(c),
                _ => return Err(ChordError::UnknownKey(name.to_string())),
            }
        }
    };
    Ok(key)
}

impl fmt::Display for Chord {
    /// Canonical form: `Ctrl+Shift+Alt+Meta+` in that order, then the key (letters uppercase).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, name) in [(self.mods.ctrl, "Ctrl"), (self.mods.shift, "Shift"), (self.mods.alt, "Alt"), (self.mods.meta, "Meta")] {
            if held {
                write!(f, "{name}+")?;
            }
        }
        let name = match self.key {
            Key::Char(c) => return write!(f, "{}", c.to_uppercase()),
            Key::F(n) => return write!(f, "F{n}"),
            Key::Up => "Up",
            Key::Down => "Down",
            Key::Left => "Left",
            Key::Right => "Right",
            Key::Enter => "Enter",
            Key::Escape => "Esc",
            Key::Delete => "Delete",
            Key::Backspace => "Backspace",
            Key::Tab => "Tab",
            Key::Home => "Home",
            Key::End => "End",
            Key::PageUp => "PageUp",
            Key::PageDown => "PageDown",
        };
        f.write_str(name)
    }
}

#[cfg(test)]
mod tests {
    use super::{Chord, ChordError, Key, Mods};

    fn ctrl() -> Mods {
        Mods { ctrl: true, ..Mods::default() }
    }

    fn parse(s: &str) -> Chord {
        s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn parses_modifiers_and_keys() {
        assert_eq!(parse("Ctrl+N"), Chord::new(Key::Char('n'), ctrl()));
        assert_eq!(parse("Ctrl+Shift+N"), Chord::new(Key::Char('n'), Mods { ctrl: true, shift: true, ..Mods::default() }));
        assert_eq!(parse("Alt+Meta+x"), Chord::new(Key::Char('x'), Mods { alt: true, meta: true, ..Mods::default() }));
        assert_eq!(parse("Ctrl+1"), Chord::new(Key::Char('1'), ctrl()));
        assert_eq!(parse("Up"), Chord::new(Key::Up, Mods::default()));
    }

    #[test]
    fn modifier_aliases() {
        assert_eq!(parse("Control+N"), parse("Ctrl+N"));
        assert_eq!(parse("Super+N"), parse("Meta+N"));
        assert_eq!(parse("Cmd+N"), parse("Meta+N"));
    }

    #[test]
    fn named_keys_and_aliases() {
        let cases = [
            ("Up", Key::Up),
            ("Down", Key::Down),
            ("Left", Key::Left),
            ("Right", Key::Right),
            ("Enter", Key::Enter),
            ("Return", Key::Enter),
            ("Esc", Key::Escape),
            ("Escape", Key::Escape),
            ("Delete", Key::Delete),
            ("Del", Key::Delete),
            ("Backspace", Key::Backspace),
            ("Tab", Key::Tab),
            ("Home", Key::Home),
            ("End", Key::End),
            ("PageUp", Key::PageUp),
            ("PageDown", Key::PageDown),
        ];
        for (name, key) in cases {
            assert_eq!(parse(name).key, key, "{name}");
        }
    }

    #[test]
    fn names_are_case_insensitive_and_trimmed() {
        assert_eq!(parse(" ctrl + shift + n "), parse("Ctrl+Shift+N"));
        assert_eq!(parse("ESC"), parse("Esc"));
        assert_eq!(parse("ctrl+enter"), parse("Ctrl+Enter"));
    }

    #[test]
    fn chord_new_lowercases_letters() {
        assert_eq!(Chord::new(Key::Char('N'), ctrl()), Chord::new(Key::Char('n'), ctrl()));
        assert_eq!(Chord::new(Key::Char('1'), ctrl()).key, Key::Char('1'));
    }

    #[test]
    fn invalid_chords_are_errors() {
        assert_eq!("".parse::<Chord>(), Err(ChordError::Empty));
        assert_eq!("Ctrl+".parse::<Chord>(), Err(ChordError::Empty));
        assert_eq!("Foo".parse::<Chord>(), Err(ChordError::UnknownKey("Foo".into())));
        assert_eq!("Hyper+N".parse::<Chord>(), Err(ChordError::UnknownModifier("Hyper".into())));
        assert_eq!("Ctrl+NN".parse::<Chord>(), Err(ChordError::UnknownKey("NN".into())));
    }

    #[test]
    fn display_is_canonical() {
        assert_eq!(parse("shift+ctrl+n").to_string(), "Ctrl+Shift+N");
        assert_eq!(parse("meta+alt+shift+ctrl+x").to_string(), "Ctrl+Shift+Alt+Meta+X");
        assert_eq!(parse("escape").to_string(), "Esc");
        assert_eq!(parse("ctrl+return").to_string(), "Ctrl+Enter");
        assert_eq!(parse("Ctrl+1").to_string(), "Ctrl+1");
    }

    #[test]
    fn display_round_trips() {
        for s in ["Ctrl+N", "Ctrl+Shift+N", "Up", "Esc", "Ctrl+Enter", "Delete", "Alt+PageDown", "Meta+Tab", "Ctrl+9"] {
            assert_eq!(parse(&parse(s).to_string()), parse(s), "{s}");
        }
    }

    #[test]
    fn function_keys_parse_and_print() {
        assert_eq!(parse("F1"), Chord::new(Key::F(1), Mods::default()));
        assert_eq!(parse("ctrl+f12"), Chord::new(Key::F(12), ctrl()));
        assert_eq!(parse("F1").to_string(), "F1");
        assert_eq!(parse("Ctrl+F12").to_string(), "Ctrl+F12");
        // "F" sozinho continua sendo a letra
        assert_eq!(parse("Ctrl+F"), Chord::new(Key::Char('f'), ctrl()));
    }

    #[test]
    fn function_keys_out_of_range_are_unknown() {
        for s in ["F0", "F13", "F99"] {
            assert_eq!(s.parse::<Chord>(), Err(ChordError::UnknownKey(s.to_string())), "{s}");
        }
    }
}
