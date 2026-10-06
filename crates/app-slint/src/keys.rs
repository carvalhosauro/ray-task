//! Atalhos globais do teclado: fonte única para o app e para a tabela do README.

use gus_keys::Keymap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    NewTask,
    NewProject,
    MoveSelection(i32),
    ExpandSelected,
    Escape,
    ToggleSelected,
    DateSelected,
    TagSelected,
    DeleteSelected,
    Undo,
    SelectNav(usize),
    ToggleFilter,
}

/// Uma linha da tabela de atalhos do README: os atalhos que ela documenta e o texto exato.
pub struct HelpRow {
    pub chords: &'static [&'static str],
    pub label: &'static str,
    pub text: &'static str,
}

use KeyAction::*;

/// Atalho → ação, na ordem da tabela do README.
const BINDINGS: &[(&str, KeyAction)] = &[
    ("Ctrl+N", NewTask),
    ("Ctrl+Shift+N", NewProject),
    ("Up", MoveSelection(-1)),
    ("Down", MoveSelection(1)),
    ("Enter", ExpandSelected),
    ("Esc", Escape),
    ("Ctrl+Enter", ToggleSelected),
    ("Ctrl+D", DateSelected),
    ("Ctrl+T", TagSelected),
    ("Delete", DeleteSelected),
    ("Ctrl+Z", Undo),
    ("Ctrl+1", SelectNav(0)),
    ("Ctrl+2", SelectNav(1)),
    ("Ctrl+3", SelectNav(2)),
    // Ctrl+4..9: projetos na ordem da sidebar (índices 3..8 em `nav`).
    ("Ctrl+4", SelectNav(3)),
    ("Ctrl+5", SelectNav(4)),
    ("Ctrl+6", SelectNav(5)),
    ("Ctrl+7", SelectNav(6)),
    ("Ctrl+8", SelectNav(7)),
    ("Ctrl+9", SelectNav(8)),
    ("Ctrl+F", ToggleFilter),
];

pub const HELP: &[HelpRow] = &[
    HelpRow { chords: &["Ctrl+N"], label: "`Ctrl+N`", text: "New task in the current view" },
    HelpRow { chords: &["Ctrl+Shift+N"], label: "`Ctrl+Shift+N`", text: "New project" },
    HelpRow { chords: &["Up", "Down"], label: "`↑` / `↓`", text: "Move between tasks" },
    HelpRow { chords: &["Enter", "Esc"], label: "`Enter` / `Esc`", text: "Open / close" },
    HelpRow { chords: &["Ctrl+Enter"], label: "`Ctrl+Enter`", text: "Complete / uncomplete" },
    HelpRow {
        chords: &["Ctrl+D"],
        label: "`Ctrl+D`",
        text: "Due date (in the popover: `H` today, `A` tomorrow, `S` next week, `Delete` no date, or type a time)",
    },
    HelpRow { chords: &["Ctrl+T"], label: "`Ctrl+T`", text: "Tag (`Enter` uses what you typed, `Tab` accepts the suggestion)" },
    HelpRow { chords: &["Delete"], label: "`Delete`", text: "Delete (with Undo)" },
    HelpRow { chords: &["Ctrl+Z"], label: "`Ctrl+Z`", text: "Undo" },
    HelpRow { chords: &["Ctrl+1", "Ctrl+2", "Ctrl+3"], label: "`Ctrl+1` / `2` / `3`", text: "Today / Upcoming / Inbox" },
    HelpRow {
        chords: &["Ctrl+4", "Ctrl+5", "Ctrl+6", "Ctrl+7", "Ctrl+8", "Ctrl+9"],
        label: "`Ctrl+4` … `Ctrl+9`",
        text: "Projects, in sidebar order",
    },
    HelpRow { chords: &["Ctrl+F"], label: "`Ctrl+F`", text: "Filter the current view" },
];

/// Os atalhos globais. Um atalho inválido ou repetido é erro de programação (o teste `keys` pega).
pub fn keymap() -> Keymap<KeyAction> {
    let mut map = Keymap::new();
    for (chord, action) in BINDINGS {
        map.bind(chord, *action).unwrap_or_else(|e| panic!("atalho {chord}: {e}"));
    }
    map
}
