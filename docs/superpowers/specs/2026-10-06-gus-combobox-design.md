# gus-combobox — design

Date: 2026-10-06
Status: draft, awaiting review

## Goal

Add a headless, std-only combobox crate — `gus-combobox` — and use it to give ray-task's tag
field a real suggestion list: the project's tags, filtered as you type, navigable with ↑/↓,
pickable with Enter or a click. Fourth GusStack crate, and the first one that changes the UI
instead of only reorganising code.

Today the tag field (`TagInput` in `crates/app-slint/ui/task_row.slint`) is an inline
autocomplete: one grey suggestion after the typed text (the first project tag that starts with
it), `Tab` completes it, `Enter` uses the typed text, `Backspace` on an empty field removes the
last tag. You cannot see the project's tags or choose among several. Extracting only that would
be ~15 lines (the same trap as `gus-undo`); the value of a combobox crate is the list state
machine, so v1 builds it and dogfoods all of it.

Success criteria:

- `crates/gus-combobox` has zero dependencies (std only).
- The tag field shows a list of up to 6 matching project tags; ↑/↓ move a highlight; Enter picks
  the highlighted tag or, with nothing highlighted, uses the typed text (as today); a click picks
  a row; Esc closes the list first.
- `Tab` completion, Enter-with-nothing-highlighted and `Backspace`-on-empty behave as today:
  `crates/app-slint/tests/ui_tag_enter.rs` and every other existing `tests/ui_*.rs` pass unchanged.
- `cargo test --workspace` green, line coverage ≥ 93%, clippy / fmt / deny clean.
- Manual check: the list opens, filters, navigates, accepts clicks and does not hide anything
  important in the task row.

## Decisions (from brainstorming)

| Decision | Choice | Why |
|---|---|---|
| Scope | Full combobox; ray-task gets the list | The only option where the crate has substance and all of it is proven in use |
| Enter | Nothing highlighted by default; Enter uses the typed text; ↓ then Enter picks | Keeps today's Enter; one simple model: type+Enter = text, ↓+Enter = list, Tab = complete |
| "Create «x»" row | Not in v1 | Redundant: Enter already creates from the typed text |
| Ranking | Starts-with first, then contains; stable within each group | Today only starts-with matches; contains helps find `#ui-kit` by typing `kit` |
| Inline grey text | Only from a starts-with match | Keeps `Tab` exactly as today |
| State shape | `Combobox { open, highlighted }`; query and options stay with the app | Same spirit as `gus-list`: the app owns its data, the crate owns the rules |

Out of scope: the "Create «x»" row, fuzzy matching, multi-select, async options, mouse hover
highlighting, publishing.

## Architecture

```
crates/gus-combobox/             # std only
  src/lib.rs                     # crate docs + re-exports
  src/matching.rs                # matches, completion
  src/combobox.rs                # Combobox, Nav, Outcome
  tests/boundary.rs              # no-dependencies rule (same detector as gus-list)
```

Dependency direction: `app-slint → gus-combobox`. `publish = false`; `[[package]]` entry in
`release-plz.toml` with `changelog_update = false`, `git_tag_enable = false`. No Slint adapter
crate: the glue is a handful of callbacks specific to ray-task's field.

## `gus-combobox`

### Matching

```rust
/// Options matching `query`, case-insensitive, query trimmed: first those whose label STARTS
/// with the query, then those that only CONTAIN it; input order kept within each group.
/// Empty query → every option.
pub fn matches<'a, T>(options: impl IntoIterator<Item = &'a T>, query: &str, label: impl Fn(&T) -> &str) -> Vec<&'a T>;

/// Inline grey completion: index into `matches` of the first match whose label STARTS with the
/// query (never one that only contains it). Empty query → None.
pub fn completion<T>(matches: &[&T], query: &str, label: impl Fn(&T) -> &str) -> Option<usize>;
```

Case folding uses `str::to_lowercase` on both sides.

### State machine

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Combobox { open: bool, highlighted: Option<usize> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav { Down, Up, Enter, Escape, Tab }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome { Ignored, Moved, Pick(usize), UseText, Complete(usize), Closed }

impl Combobox {
    pub fn open(&mut self);            // field focused: open, nothing highlighted
    pub fn close(&mut self);           // field blurred: closed, nothing highlighted
    pub fn input(&mut self);           // text changed: open, nothing highlighted
    pub fn key(&mut self, nav: Nav, matches: usize, completion: Option<usize>) -> Outcome;
    pub fn is_open(&self) -> bool;
    pub fn highlighted(&self) -> Option<usize>;
}
```

`key` rules (`matches` = number of matches currently shown):

| Key | State | Outcome and new state |
|---|---|---|
| Down | `matches == 0` | `Ignored` |
| Down | nothing highlighted | open, highlight 0, `Moved` |
| Down | highlighted `i` | highlight `min(i + 1, matches - 1)`, `Moved` |
| Up | nothing highlighted | `Ignored` |
| Up | highlighted 0 | nothing highlighted, `Moved` |
| Up | highlighted `i > 0` | highlight `min(i - 1, matches - 1)`, `Moved` |
| Enter | highlighted `i < matches` | `Pick(i)`, closed, nothing highlighted |
| Enter | otherwise | `UseText`, closed, nothing highlighted |
| Tab | `completion == Some(i)` | `Complete(i)`, nothing highlighted |
| Tab | `completion == None` | `Ignored` |
| Escape | open | `Closed`, nothing highlighted |
| Escape | closed | `Ignored` |

A highlight beyond the current `matches` (the list shrank) is clamped on Up/Down and treated as
nothing highlighted on Enter. No function returns `Result` or panics.

## ray-task integration

### Controller

- New field `tag_combo: Combobox` (only one tag field has focus at a time).
- `MAX_TAG_OPTIONS = 6`.
- `tag_options(&self, id, query) -> Vec<String>`: `matches` over the project tags the task does
  not have yet, query normalised as today (trim, strip a leading `#`), capped at 6.
- `tag_suggestion(&self, id, query) -> String`: now `completion` over the same options; same
  results as today for every existing test (starts-with only).
- `tag_focus(&mut self, focused: bool)`, `tag_input(&mut self)`: forward to `open` / `close` /
  `input`.
- `tag_highlighted(&self) -> Option<usize>`, `tag_list_open(&self) -> bool`.
- `tag_key(&mut self, id, query, nav) -> TagKey` where `pub struct TagKey { pub handled: bool, pub text: String }`:
  - `Pick(i)` → `add_tag_by_name(option i)`; handled; text `""`.
  - `UseText` → `add_tag_by_name(query)` (empty query: nothing added); handled; text `""`.
  - `Complete(i)` → handled; text = option `i`'s name.
  - `Moved` / `Closed` → handled; text unchanged.
  - `Ignored` → not handled; text unchanged (Slint lets the event through).
  - Errors from `add_tag_by_name` are returned to `bind`, which logs them with `log_err` as today.

### Slint

`globals.slint` gains:

```slint
in-out property <int> tag-highlighted: -1;
in-out property <bool> tag-list-open;
pure callback tag-options(int, string) -> [string];
callback tag-focus(bool);
callback tag-input();
callback tag-key(int, string, string) -> TagKey;   // key: "down" | "up" | "enter" | "escape" | "tab"
callback tag-pick(int, string);
```

with `struct TagKey { handled: bool, text: string }` in `types.slint`.

`TagInput`:
- `key-pressed`: `Backspace` on an empty field stays as today (`remove-last-tag`). `UpArrow`,
  `DownArrow`, `Return`, `Escape`, `Tab` call `Actions.tag-key(...)`; if `handled`, set
  `self.text = result.text` and accept, else reject.
- The `accepted` handler is removed (Enter now goes through `tag-key`).
- `changed has-focus` → `Actions.tag-focus(self.has-focus)`; `edited` → `Actions.tag-input()`.
- The grey inline text keeps using `tag-suggest`.
- A list below the field, visible when the field has focus, `Actions.tag-list-open` and options
  are non-empty: up to 6 rows of 26 px showing `#name`, same look as the existing tag menu
  (`Theme` colours, rounded corners); the row at `Actions.tag-highlighted` gets the accent
  background; clicking a row calls `Actions.tag-pick(task, name)` and clears the field.

### `bind.rs`

Wires the callbacks to the controller; after `tag-focus`, `tag-input`, `tag-key` and `tag-pick`
it sets `tag-highlighted` / `tag-list-open` from the controller and refreshes rows when a tag
was added (as `add-tag` does today).

### Intended behaviour changes

- Focusing the tag field (Ctrl+T or click) shows the project's tags.
- Typing `kit` now lists `#ui-kit` (contains-match); the grey inline text still only completes
  starts-with matches.
- Esc with the list open closes only the list; the next Esc reaches the app's Esc layers.

## Error handling

`gus-combobox` is total. In ray-task, `tag_key` / `tag-pick` propagate `DomainError` from
`add_tag_by_name` to `bind`, which logs it with `log_err` and leaves the field text unchanged,
as the `add-tag` path does today.

## Testing

- `gus-combobox`: ranking (starts-with before contains, stable order, case, surrounding spaces,
  empty query, no match), `completion` (never a contains-only match, empty query → None), every
  row of the `key` table, clamping after the list shrinks, `open` / `close` / `input` resets.
- Boundary test identical to `gus-list`'s.
- `tests/controller.rs`: options exclude the task's tags, honour `#` and spaces, are capped at 6,
  rank starts-with first; `tag_suggestion` unchanged; `tag_key` for Pick / UseText / Complete /
  Moved / Closed / Ignored, and Pick / UseText add the tag.
- New `tests/ui_tag_combobox.rs` (real key events, mocked time): Ctrl+T shows the list; type
  `ca`, ↓, Enter → `casa` added; with the list open Esc closes it and the next Esc collapses the
  task; clicking a row adds that tag.
- `tests/ui_tag_enter.rs` and all other `tests/ui_*.rs` unchanged.

## Migration order

1. `crates/gus-combobox`: `matches`, `completion`, `Combobox` and tests.
2. Controller: `tag_combo`, `tag_options`, `tag_key`, focus/input, `tag_suggestion` on
   `completion`; controller tests.
3. Slint + `bind`: callbacks, `TagKey`, the list popup, `ui_tag_combobox.rs`; existing UI tests
   unchanged; full verification; manual check.
