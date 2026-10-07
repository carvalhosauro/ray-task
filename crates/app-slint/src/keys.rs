//! Atalhos globais do teclado: fonte única para o app, a ajuda no app e a tabela do README.

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
    OpenSettings,
    ToggleHelp,
}

/// Uma linha da tabela de atalhos: os atalhos que ela documenta, o rótulo, o texto do README
/// (inglês) e o texto mostrado no app (português).
pub struct HelpRow {
    pub chords: &'static [&'static str],
    pub label: &'static str,
    pub text: &'static str,
    pub pt: &'static str,
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
    ("Ctrl+,", OpenSettings),
    ("F1", ToggleHelp),
    ("?", ToggleHelp),
];

pub const HELP: &[HelpRow] = &[
    HelpRow { chords: &["Ctrl+N"], label: "`Ctrl+N`", text: "New task in the current view", pt: "Nova tarefa na visão atual" },
    HelpRow { chords: &["Ctrl+Shift+N"], label: "`Ctrl+Shift+N`", text: "New project", pt: "Novo projeto" },
    HelpRow { chords: &["Up", "Down"], label: "`↑` / `↓`", text: "Move between tasks", pt: "Mover entre tarefas" },
    HelpRow { chords: &["Enter", "Esc"], label: "`Enter` / `Esc`", text: "Open / close", pt: "Abrir / fechar" },
    HelpRow { chords: &["Ctrl+Enter"], label: "`Ctrl+Enter`", text: "Complete / uncomplete", pt: "Concluir / reabrir" },
    HelpRow {
        chords: &["Ctrl+D"],
        label: "`Ctrl+D`",
        text: "Due date (in the popover: `H` today, `A` tomorrow, `S` next week, `Delete` no date, or type a time)",
        pt: "Prazo (no popover: `H` hoje, `A` amanhã, `S` próxima semana, `Delete` sem data, ou digite uma hora)",
    },
    HelpRow {
        chords: &["Ctrl+T"],
        label: "`Ctrl+T`",
        text: "Tag (`Enter` uses what you typed, `Tab` accepts the suggestion)",
        pt: "Tag (`Enter` usa o que você digitou, `Tab` aceita a sugestão)",
    },
    HelpRow { chords: &["Delete"], label: "`Delete`", text: "Delete (with Undo)", pt: "Apagar (com Desfazer)" },
    HelpRow { chords: &["Ctrl+Z"], label: "`Ctrl+Z`", text: "Undo", pt: "Desfazer" },
    HelpRow {
        chords: &["Ctrl+1", "Ctrl+2", "Ctrl+3"],
        label: "`Ctrl+1` / `2` / `3`",
        text: "Today / Upcoming / Inbox",
        pt: "Hoje / Próximos / Entrada",
    },
    HelpRow {
        chords: &["Ctrl+4", "Ctrl+5", "Ctrl+6", "Ctrl+7", "Ctrl+8", "Ctrl+9"],
        label: "`Ctrl+4` … `Ctrl+9`",
        text: "Projects, in sidebar order",
        pt: "Projetos, na ordem da barra lateral",
    },
    HelpRow { chords: &["Ctrl+F"], label: "`Ctrl+F`", text: "Filter the current view", pt: "Filtrar a visão atual" },
    HelpRow { chords: &["Ctrl+,"], label: "`Ctrl+,`", text: "Settings", pt: "Configurações" },
    HelpRow { chords: &["F1", "?"], label: "`F1` / `?`", text: "Shortcut list", pt: "Esta lista de atalhos" },
];

/// Linhas da ajuda no app: (atalhos, texto em português), sem as crases do markdown.
pub fn help_items() -> Vec<(String, String)> {
    HELP.iter().map(|row| (row.label.replace('`', ""), row.pt.replace('`', ""))).collect()
}

/// Os atalhos globais. Um atalho inválido ou repetido é erro de programação (o teste `keys` pega).
pub fn keymap() -> Keymap<KeyAction> {
    let mut map = Keymap::new();
    for (chord, action) in BINDINGS {
        map.bind(chord, *action).unwrap_or_else(|e| panic!("atalho {chord}: {e}"));
    }
    map
}
