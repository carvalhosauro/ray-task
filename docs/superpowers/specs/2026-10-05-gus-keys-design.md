# gus-keys — design

Date: 2026-10-05
Status: draft, awaiting review

## Goal

Make ray-task's global keyboard shortcuts a single source of truth in Rust, backed by a
headless, std-only crate — `gus-keys` (chords + keymap) — plus a small Slint adapter,
`gus-keys-slint`. Third GusStack crate after `gus-list` and `gus-anim-state`.

Today the global shortcuts live in one `key-pressed` handler in `crates/app-slint/ui/app.slint`
(about 20 `if`s), and the README's shortcut table is maintained by hand next to it. Problems:

1. Shortcuts cannot be tested without driving the UI.
2. The README can drift from the code, and nothing catches it.
3. There is no list of shortcuts the app can use (future help screen, conflict detection,
   configurable shortcuts).

v1 is **dogfood-first**: exactly ray-task's current global shortcuts.

Success criteria:

- `crates/gus-keys` has zero dependencies (std only).
- The global `key-pressed` handler in `app.slint` is one line that forwards to Rust.
- A test proves the README shortcut table and the keymap describe the same set of chords.
- Every `crates/app-slint/tests/ui_*.rs` test passes **unchanged** (they dispatch real key
  events).
- `cargo test --workspace` green, line coverage ≥ 93%, clippy / fmt / deny clean.
- Manual check: every shortcut in the README table works in the running app.

## Decisions (from brainstorming)

| Decision | Choice | Why |
|---|---|---|
| Scope | Keymap as single source in Rust; field-local keys stay in Slint | Attacks all three problems; field keys depend on focus and the field's text |
| Esc layers | Not in the crate | `Controller::escape` is a clear, tested 5-step chain; a crate would be ~40 lines (same reason `gus-undo` was deferred) |
| README sync | Help rows as data in ray-task + tests | Keeps the README's grouped rows as written; catches drift in CI; reusable for a help screen |
| Modifier matching | Exact | Today only `N` checks Shift; others ignore extra modifiers. Exact matching frees `Ctrl+Shift+Z` for a future redo |
| Slint glue | Adapter crate `gus-keys-slint` | Converting Slint's special-key codes is reusable by any Slint app |

Out of scope: Esc layers, field-local shortcuts (`task_row.slint`), user-configurable
shortcuts, an in-app help screen, multi-key sequences (`g g`), publishing, a GitHub org.

## Architecture

```
crates/gus-keys/                 # std only
  src/lib.rs                     # crate docs + re-exports
  src/chord.rs                   # Key, Mods, Chord, ChordError (FromStr / Display)
  src/keymap.rs                  # Keymap<A>, KeymapError
  tests/boundary.rs              # no-dependencies rule (same detector as gus-list)
crates/gus-keys-slint/           # deps: gus-keys, slint
  src/lib.rs                     # chord_from_slint
crates/app-slint/src/keys.rs     # KeyAction, keymap(), HELP
```

Dependency direction: `app-slint → gus-keys-slint → gus-keys`, `app-slint → gus-keys`. Both new
crates are `publish = false` with `[[package]]` entries in `release-plz.toml`
(`changelog_update = false`, `git_tag_enable = false`).

## `gus-keys`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key { Char(char), Up, Down, Left, Right, Enter, Escape, Delete, Backspace, Tab, Home, End, PageUp, PageDown }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods { pub ctrl: bool, pub shift: bool, pub alt: bool, pub meta: bool }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord { pub key: Key, pub mods: Mods }

impl Chord { pub fn new(key: Key, mods: Mods) -> Self; }   // lowercases Key::Char
impl FromStr for Chord { type Err = ChordError; }
impl Display for Chord;

pub struct Keymap<A>;
impl<A> Keymap<A> {
    pub fn new() -> Self;                                                   // also Default
    pub fn bind(&mut self, chord: &str, action: A) -> Result<(), KeymapError>;
    pub fn lookup(&self, chord: &Chord) -> Option<&A>;
    pub fn chords(&self) -> impl Iterator<Item = &Chord>;
}

pub enum ChordError { Empty, UnknownKey(String), UnknownModifier(String) }  // Display + Error
pub enum KeymapError { Parse(ChordError), Conflict(Chord) }                  // Display + Error
```

Chord syntax:

- `+`-separated, modifiers first, key last; whitespace around parts ignored; case-insensitive
  names. Modifiers: `Ctrl` / `Control`, `Shift`, `Alt`, `Meta` / `Super` / `Cmd`.
- Named keys: `Up`, `Down`, `Left`, `Right`, `Enter` / `Return`, `Esc` / `Escape`, `Delete` /
  `Del`, `Backspace`, `Tab`, `Home`, `End`, `PageUp`, `PageDown`. Any other single character is
  `Key::Char`, lowercased (`N` → `n`). Anything else is `UnknownKey`.
- `Display` is canonical: modifiers in the order `Ctrl+Shift+Alt+Meta`, then the key (letters
  uppercase, named keys by their first spelling above, e.g. `Ctrl+Shift+N`, `Esc`).
- `Chord::new` lowercases `Key::Char`, so `Chord::new(Key::Char('N'), ctrl)` equals
  `"Ctrl+N".parse()`: Caps Lock or a platform that reports `N` without Shift still matches.
- Matching is **exact**: `Ctrl+Shift+Z` does not match a `Ctrl+Z` binding.

`Keymap::bind` fails on a parse error or when the chord is already bound (`Conflict`, reporting
the chord). `Keymap` is a `Vec<(Chord, A)>` in insertion order (`chords()` follows it); lookup
is a linear scan, which is fine for a few dozen bindings.

This is the one GusStack API that returns `Result`: its input is text. Apps build their keymap at
startup with `expect` and a test builds the same keymap, so a typo fails CI, never a user.

## `gus-keys-slint`

```rust
pub fn chord_from_slint(text: &str, ctrl: bool, shift: bool, alt: bool, meta: bool) -> Option<Chord>;
```

- `text` is Slint's `KeyEvent.text`. Slint's special keys (`slint::platform::Key::UpArrow`,
  `DownArrow`, `LeftArrow`, `RightArrow`, `Return`, `Escape`, `Delete`, `Backspace`, `Tab`,
  `Home`, `End`, `PageUp`, `PageDown`) map to the matching `Key`.
- Any other single character becomes `Key::Char` (via `Chord::new`, so lowercased).
- Empty text, multi-character text, and modifier-only keys (`Key::Control`, `Shift`, `Alt`,
  `Meta`, …) return `None`.

## ray-task integration

### `crates/app-slint/src/keys.rs`

```rust
pub enum KeyAction {
    NewTask, NewProject, MoveSelection(i32), ExpandSelected, Escape, ToggleSelected,
    DateSelected, TagSelected, DeleteSelected, Undo, SelectNav(usize), ToggleFilter,
}

pub fn keymap() -> Keymap<KeyAction>;

pub struct HelpRow { pub chords: &'static [&'static str], pub label: &'static str, pub text: &'static str }
pub const HELP: &[HelpRow];
```

`keymap()` binds the 20 global shortcuts that exist today: `Ctrl+N`, `Ctrl+Shift+N`, `Up`,
`Down`, `Enter`, `Esc`, `Ctrl+Enter`, `Ctrl+D`, `Ctrl+T`, `Delete`, `Ctrl+Z`, `Ctrl+1`…`Ctrl+9`
(`SelectNav(0..=8)`), `Ctrl+F`.

`HELP` holds the 12 rows of the README table with identical `label` / `text`, e.g.
`HelpRow { chords: &["Ctrl+1", "Ctrl+2", "Ctrl+3"], label: "`Ctrl+1` / `2` / `3`", text: "Today / Upcoming / Inbox" }`.

### Slint

`globals.slint` gains `callback key(string, bool, bool, bool, bool) -> bool;` and the global
`key-pressed` in `app.slint` becomes:

```slint
key-pressed(event) => {
    return Actions.key(event.text, event.modifiers.control, event.modifiers.shift, event.modifiers.alt, event.modifiers.meta) ? accept : reject;
}
```

`task_row.slint`'s field handlers are unchanged.

### `bind.rs`

`on_key` builds the chord with `chord_from_slint`, looks it up in a keymap built once
(`keys::keymap()` stored in the wiring closure), and calls the existing `Actions` callback for the
action (`invoke_new_task()`, `invoke_move_selection(delta)`, `invoke_select_nav(i)`, …), then
returns `true`. No match → `false` (Slint rejects, the event propagates as today). The existing
`on_*` handlers do not change.

### Intended behaviour change

Today's handler ignores extra modifiers except Shift on `N`, so `Ctrl+Shift+Z` undoes,
`Ctrl+Shift+F` filters, `Ctrl+Alt+N` creates a task, `Shift+↑` moves the selection, and so on.
With exact matching these no longer trigger anything. A test pins `Ctrl+Shift+Z → None`.

## Error handling

`gus-keys`: parsing returns `ChordError`; `bind` returns `KeymapError`; `lookup` and
`chord_from_slint` are total (`None` when nothing matches). No panics in library code. ray-task:
`keys::keymap()` uses `expect` on each `bind` (programmer error, caught by the test that builds
it).

## Testing

- `gus-keys`: parse of every modifier and named-key alias, case and whitespace, single chars and
  digits, invalid input (`""`, `"Ctrl+"`, `"Foo"`, `"Hyper+N"`); canonical `Display` and
  `parse(display(c)) == c` round trip; `Chord::new` lowercases; keymap lookup, exact modifier
  match, conflict, `chords()` order.
- Boundary test identical to `gus-list`'s.
- `gus-keys-slint`: each Slint special key maps to the right `Key`; plain and uppercase letters;
  empty, multi-char and modifier-only text → `None`.
- `crates/app-slint/tests/keys.rs`: `keymap()` builds; the set of `HELP` chords equals the
  keymap's chords (both directions, parsed so spelling differences do not matter); `README.md`
  contains every `| label | text |` row; `Ctrl+N` ≠ `Ctrl+Shift+N`; uppercase `N` without Shift
  matches `Ctrl+N`; `Ctrl+Shift+Z`, `Ctrl+0` and an unbound key → `None`.
- `crates/app-slint/tests/ui_*.rs` unchanged.

## Migration order

1. `crates/gus-keys` with `Chord`, `Keymap` and tests.
2. `crates/gus-keys-slint` with `chord_from_slint` and tests.
3. ray-task: `keys.rs` + `tests/keys.rs`, the `key` callback, `on_key` in `bind.rs`, the one-line
   `key-pressed`; UI tests unchanged, full suite, coverage, deny, manual check.
