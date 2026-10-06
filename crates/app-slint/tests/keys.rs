use std::collections::HashSet;

use gus_keys::{Chord, Key};
use ray_task::keys::{keymap, KeyAction, HELP};

fn chord(s: &str) -> Chord {
    s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
}

#[test]
fn keymap_has_every_global_shortcut() {
    let map = keymap();
    assert_eq!(map.chords().count(), 24);
    assert_eq!(map.lookup(&chord("Ctrl+,")), Some(&KeyAction::OpenSettings));
    assert_eq!(map.lookup(&chord("F1")), Some(&KeyAction::ToggleHelp));
    assert_eq!(map.lookup(&chord("?")), Some(&KeyAction::ToggleHelp));
    assert_eq!(map.lookup(&chord("Ctrl+N")), Some(&KeyAction::NewTask));
    assert_eq!(map.lookup(&chord("Ctrl+Shift+N")), Some(&KeyAction::NewProject));
    assert_eq!(map.lookup(&chord("Up")), Some(&KeyAction::MoveSelection(-1)));
    assert_eq!(map.lookup(&chord("Down")), Some(&KeyAction::MoveSelection(1)));
    assert_eq!(map.lookup(&chord("Enter")), Some(&KeyAction::ExpandSelected));
    assert_eq!(map.lookup(&chord("Esc")), Some(&KeyAction::Escape));
    assert_eq!(map.lookup(&chord("Ctrl+Enter")), Some(&KeyAction::ToggleSelected));
    assert_eq!(map.lookup(&chord("Ctrl+D")), Some(&KeyAction::DateSelected));
    assert_eq!(map.lookup(&chord("Ctrl+T")), Some(&KeyAction::TagSelected));
    assert_eq!(map.lookup(&chord("Delete")), Some(&KeyAction::DeleteSelected));
    assert_eq!(map.lookup(&chord("Ctrl+Z")), Some(&KeyAction::Undo));
    assert_eq!(map.lookup(&chord("Ctrl+F")), Some(&KeyAction::ToggleFilter));
    for n in 1..=9 {
        assert_eq!(map.lookup(&chord(&format!("Ctrl+{n}"))), Some(&KeyAction::SelectNav(n - 1)));
    }
}

#[test]
fn uppercase_letter_without_shift_is_still_ctrl_n() {
    let chord = gus_keys_slint::chord_from_slint("N", true, false, false, false).unwrap();
    assert_eq!(keymap().lookup(&chord), Some(&KeyAction::NewTask));
}

#[test]
fn extra_modifiers_do_not_match() {
    let map = keymap();
    for s in ["Ctrl+Shift+Z", "Ctrl+Shift+F", "Ctrl+Alt+N", "Shift+Up", "Alt+Delete"] {
        assert_eq!(map.lookup(&chord(s)), None, "{s}");
    }
}

#[test]
fn unbound_keys_are_not_handled() {
    let map = keymap();
    for s in ["Ctrl+0", "Ctrl+Q", "N", "Left", "Tab"] {
        assert_eq!(map.lookup(&chord(s)), None, "{s}");
    }
}

#[test]
fn help_rows_cover_exactly_the_keymap() {
    let bound: HashSet<Chord> = keymap().chords().copied().collect();
    let documented: HashSet<Chord> = HELP.iter().flat_map(|row| row.chords.iter().map(|s| chord(s))).collect();
    let undocumented: Vec<String> = bound.difference(&documented).map(ToString::to_string).collect();
    let unbound: Vec<String> = documented.difference(&bound).map(ToString::to_string).collect();
    assert!(undocumented.is_empty(), "atalhos sem linha de ajuda: {undocumented:?}");
    assert!(unbound.is_empty(), "ajuda de atalhos que não existem: {unbound:?}");
}

/// Atalhos que o rótulo mostra: cada `token` é um atalho; um token sem `+` herda os modificadores
/// do anterior (`Ctrl+1` / `2`); `A` … `B` inclui a faixa de dígitos entre eles; setas são Up/Down.
fn label_chords(label: &str) -> HashSet<Chord> {
    let parts: Vec<&str> = label.split('`').collect();
    let mut out: Vec<Chord> = Vec::new();
    for (i, token) in parts.iter().enumerate().skip(1).step_by(2) {
        let token = match *token {
            "↑" => "Up",
            "↓" => "Down",
            other => other,
        };
        let mut next = chord(token);
        if let Some(previous) = out.last() {
            if !token.contains('+') {
                next.mods = previous.mods;
            }
            if parts[i - 1].contains('…') {
                let (Key::Char(from), Key::Char(to)) = (previous.key, next.key) else { panic!("faixa só de dígitos: {label}") };
                let middle: Vec<Chord> = (from..to).skip(1).map(|c| Chord::new(Key::Char(c), next.mods)).collect();
                out.extend(middle);
            }
        }
        out.push(next);
    }
    out.into_iter().collect()
}

#[test]
fn help_labels_show_exactly_their_chords() {
    for row in HELP {
        let declared: HashSet<Chord> = row.chords.iter().map(|s| chord(s)).collect();
        assert_eq!(label_chords(row.label), declared, "rótulo {} não bate com {:?}", row.label, row.chords);
    }
}

#[test]
fn readme_table_is_exactly_the_help_rows() {
    let readme = include_str!("../../../README.md");
    let section = readme.split("## Keyboard first").nth(1).expect("seção de atalhos no README");
    let rows: Vec<&str> = section.lines().skip_while(|l| !l.starts_with("|---")).skip(1).take_while(|l| l.starts_with('|')).collect();
    let expected: Vec<String> = HELP.iter().map(|row| format!("| {} | {} |", row.label, row.text)).collect();
    assert_eq!(rows, expected, "a tabela do README e `keys::HELP` divergem");
}

#[test]
fn question_mark_with_shift_still_opens_help() {
    // Em layouts US o `?` sai com Shift: o adaptador descarta o Shift de símbolos.
    let chord = gus_keys_slint::chord_from_slint("?", false, true, false, false).unwrap();
    assert_eq!(keymap().lookup(&chord), Some(&KeyAction::ToggleHelp));
}

#[test]
fn help_items_are_portuguese_without_backticks() {
    let items = ray_task::keys::help_items();
    assert_eq!(items.len(), HELP.len());
    assert_eq!(items[0], ("Ctrl+N".to_string(), "Nova tarefa na visão atual".to_string()));
    assert!(items.iter().all(|(k, t)| !k.contains('`') && !t.contains('`')));
    assert!(items.iter().any(|(k, _)| k == "F1 / ?"));
}
