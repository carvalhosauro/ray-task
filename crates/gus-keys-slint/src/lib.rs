//! Slint adapter for gus-keys: turns a `KeyEvent` (text + modifiers) into a [`gus_keys::Chord`].

use gus_keys::{Chord, Key, Mods};
use slint::platform::Key as SlintKey;

/// The chord for a Slint key event, or `None` for keys `gus-keys` does not model (modifiers
/// pressed alone, function keys, other special keys) and for empty or multi-character text.
///
/// Call it from a `FocusScope`'s `key-pressed` with `event.text` and `event.modifiers.*`.
pub fn chord_from_slint(text: &str, ctrl: bool, shift: bool, alt: bool, meta: bool) -> Option<Chord> {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else { return None };
    let key = named(c).or_else(|| is_printable(c).then_some(Key::Char(c)))?;
    Some(Chord::new(key, Mods { ctrl, shift, alt, meta }))
}

fn named(c: char) -> Option<Key> {
    [
        (SlintKey::UpArrow, Key::Up),
        (SlintKey::DownArrow, Key::Down),
        (SlintKey::LeftArrow, Key::Left),
        (SlintKey::RightArrow, Key::Right),
        (SlintKey::Return, Key::Enter),
        (SlintKey::Escape, Key::Escape),
        (SlintKey::Delete, Key::Delete),
        (SlintKey::Backspace, Key::Backspace),
        (SlintKey::Tab, Key::Tab),
        (SlintKey::Home, Key::Home),
        (SlintKey::End, Key::End),
        (SlintKey::PageUp, Key::PageUp),
        (SlintKey::PageDown, Key::PageDown),
    ]
    .into_iter()
    .find(|(slint_key, _)| char::from(*slint_key) == c)
    .map(|(_, key)| key)
}

/// Slint encodes the other special keys as control characters or in the private-use area.
fn is_printable(c: char) -> bool {
    !c.is_control() && !('\u{E000}'..='\u{F8FF}').contains(&c)
}

#[cfg(test)]
mod tests {
    use gus_keys::{Chord, Key, Mods};
    use slint::platform::Key as SlintKey;
    use slint::SharedString;

    use super::chord_from_slint;

    fn text(key: SlintKey) -> SharedString {
        key.into()
    }

    fn plain(key: Key) -> Option<Chord> {
        Some(Chord::new(key, Mods::default()))
    }

    #[test]
    fn special_keys_map_to_named_keys() {
        let cases = [
            (SlintKey::UpArrow, Key::Up),
            (SlintKey::DownArrow, Key::Down),
            (SlintKey::LeftArrow, Key::Left),
            (SlintKey::RightArrow, Key::Right),
            (SlintKey::Return, Key::Enter),
            (SlintKey::Escape, Key::Escape),
            (SlintKey::Delete, Key::Delete),
            (SlintKey::Backspace, Key::Backspace),
            (SlintKey::Tab, Key::Tab),
            (SlintKey::Home, Key::Home),
            (SlintKey::End, Key::End),
            (SlintKey::PageUp, Key::PageUp),
            (SlintKey::PageDown, Key::PageDown),
        ];
        for (slint_key, key) in cases {
            assert_eq!(chord_from_slint(&text(slint_key), false, false, false, false), plain(key), "{slint_key:?}");
        }
    }

    #[test]
    fn modifiers_are_carried() {
        let chord = chord_from_slint("n", true, true, false, false).unwrap();
        assert_eq!(chord, "Ctrl+Shift+N".parse().unwrap());
        let chord = chord_from_slint(&text(SlintKey::Return), true, false, false, false).unwrap();
        assert_eq!(chord, "Ctrl+Enter".parse().unwrap());
        assert_eq!(chord_from_slint("x", false, false, true, true).unwrap(), "Alt+Meta+X".parse().unwrap());
    }

    #[test]
    fn uppercase_letter_maps_to_lowercase_char() {
        assert_eq!(chord_from_slint("N", true, false, false, false).unwrap(), "Ctrl+N".parse().unwrap());
    }

    #[test]
    fn digits_and_symbols_are_chars() {
        assert_eq!(chord_from_slint("1", true, false, false, false).unwrap(), "Ctrl+1".parse().unwrap());
        assert_eq!(chord_from_slint("/", false, false, false, false), plain(Key::Char('/')));
    }

    #[test]
    fn modifier_only_and_function_keys_are_none() {
        for key in [SlintKey::Control, SlintKey::Shift, SlintKey::Alt, SlintKey::Meta, SlintKey::F1, SlintKey::Insert] {
            assert_eq!(chord_from_slint(&text(key), true, false, false, false), None, "{key:?}");
        }
    }

    #[test]
    fn empty_or_multi_char_text_is_none() {
        assert_eq!(chord_from_slint("", true, false, false, false), None);
        assert_eq!(chord_from_slint("ab", false, false, false, false), None);
    }
}
