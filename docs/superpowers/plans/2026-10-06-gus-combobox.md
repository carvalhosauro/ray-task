# gus-combobox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the std-only `gus-combobox` crate and give ray-task's tag field a navigable suggestion list built on it, keeping today's Enter / Tab / Backspace behaviour.

**Architecture:** `gus-combobox` ranks options (`matches`), finds the inline completion (`completion`) and runs the list state machine (`Combobox::key`). `Controller` owns one `Combobox` for the focused tag field and turns outcomes into tag changes. Slint forwards the field's focus, edits and keys to Rust; the list is drawn as an overlay in `app.slint`, positioned from the field's `absolute-position`.

**Tech Stack:** Rust 2021 (MSRV 1.92), Slint 1.18, cargo workspace, lefthook, cargo-deny, cargo-llvm-cov.

**Spec:** `docs/superpowers/specs/2026-10-06-gus-combobox-design.md`

## Global Constraints

- `crates/gus-combobox` has **no dependency tables at all**; it never references `ray-core`, `chrono`, `slint` or ray-task types.
- New crate: workspace `edition` / `version` / `rust-version` (1.92) / `license` / `repository`, `publish = false`, `[[package]]` in `release-plz.toml` with `changelog_update = false`, `git_tag_enable = false`.
- `gus-combobox` is total: no `Result`, no panics.
- At most **6** options shown.
- Every existing `crates/app-slint/tests/ui_*.rs` passes **without modification** (notably `ui_tag_enter.rs`). Existing `tests/controller.rs` tests are not modified.
- Style: `rustfmt.toml`; ray-task comments in Portuguese, crate docs in English.
- CI gates: fmt, clippy `-D warnings`, `cargo test --workspace`, MSRV 1.92, line coverage ≥ 93%, `cargo deny check`.
- Commit subjects: Conventional Commits.
- `CARGO_BUILD_JOBS=2`, one cargo command at a time.

## Planning findings (deviations from the spec's wording)

1. **The list is not a `PopupWindow`.** Slint's `PopupWindow` takes keyboard focus while open (`i-slint-core-1.18.1/window.rs:529`, `focus_item_in_parent`), which would stop typing in the tag field. The spec's "same look as the existing tag menu" is kept (`MenuPanel` styling), but the list is an overlay element at the end of `app.slint`, outside the root `keys` `FocusScope`, positioned from the field's `absolute-position` (an output property Slint 1.18 supports). Clicking a `TouchArea` does not move keyboard focus (`items/input_items.rs:787`: only `FocusScope` / `TextInput` take focus on click).
2. **Esc only consumes when a list is actually visible**: `Escape` returns `Closed` when `open && matches > 0`, otherwise `Ignored`. Without this, in a project with no tags the first Esc after Ctrl+T would do nothing visible. (Spec table: "Escape | open → Closed".)
3. **Key names cross the Slint boundary as Slint key text**, not as `"down"`/`"up"` strings: `tag-key(task, field-text, event.text, modified)`; `bind.rs` maps `slint::platform::Key` chars to `Nav`. Enter keeps working with modifiers (the old `accepted` fired on Ctrl+Enter too); other keys with Ctrl/Alt/Meta are not handled.
4. `project_tags` is sorted by name, so for tags `casa, carro, ui-kit` and query `ca` the list is `carro, casa`.

## Review Focus

1. **Tag field with no project tags** — Ctrl+T then Esc must still close the task (first Esc not swallowed). Test: `escape_without_visible_list_is_ignored` (Task 1), `escape_in_field_without_tags_reaches_the_app` (Task 3).
2. **Typing after picking with ↓/Enter** — the list must reopen with fresh matches and nothing highlighted; Enter then uses the new text. Test: `input_reopens_and_clears_highlight` (Task 1), `ui_tag_combobox` flow (Task 3).
3. **The list shrinks under a highlight** (highlight on item 4, user types and only 2 remain) — Enter must not pick an out-of-range item. Test: `enter_with_stale_highlight_uses_text` (Task 1).
4. **↑/↓ in the field when there are no matches** — must still reach the app (moving the task selection, as today). Test: `down_without_matches_is_ignored` (Task 1), `tag_key_ignored_without_options` (Task 2).
5. **Enter on an empty field** — nothing is added, field stays empty, no error toast/log. Test: `enter_on_empty_query_adds_nothing` (Task 2).

---

### Task 1: `gus-combobox` crate

**Files:** create `crates/gus-combobox/{Cargo.toml, src/lib.rs, src/matching.rs, src/combobox.rs, tests/boundary.rs}`; modify `Cargo.toml` (members), `release-plz.toml`.

**Produces:**
```rust
pub fn matches<'a, T>(options: impl IntoIterator<Item = &'a T>, query: &str, label: impl Fn(&T) -> &str) -> Vec<&'a T>;
pub fn completion<T>(matches: &[&T], query: &str, label: impl Fn(&T) -> &str) -> Option<usize>;
pub struct Combobox;   // Default; open, close, input, key, is_open, highlighted
pub enum Nav { Down, Up, Enter, Escape, Tab }
pub enum Outcome { Ignored, Moved, Pick(usize), UseText, Complete(usize), Closed }
```

- [ ] **Step 1:** manifest (`description = "Headless combobox: ranked matches, inline completion and a keyboard-driven list."`), register in workspace members after `crates/gus-keys-slint`, `release-plz.toml` entry, boundary test copied from `crates/gus-keys/tests/boundary.rs` with the crate name replaced.
- [ ] **Step 2:** write failing tests in `src/matching.rs`:
  - `empty_query_returns_everything_in_order`, `starts_with_comes_before_contains`, `order_is_stable_within_each_group`, `case_and_surrounding_spaces_are_ignored`, `no_match_is_empty`;
  - `completion_is_the_first_starts_with_match`, `completion_never_picks_a_contains_only_match`, `completion_of_empty_query_is_none`.
- [ ] **Step 3:** write failing tests in `src/combobox.rs` for every row of the spec's key table plus Review Focus 1–4: `down_from_nothing_highlights_first`, `down_clamps_at_last`, `down_without_matches_is_ignored`, `up_from_first_returns_to_text`, `up_without_highlight_is_ignored`, `up_clamps_after_list_shrank`, `enter_picks_highlighted_and_closes`, `enter_without_highlight_uses_text`, `enter_with_stale_highlight_uses_text`, `tab_completes_when_there_is_a_completion`, `tab_without_completion_is_ignored`, `escape_closes_a_visible_list`, `escape_without_visible_list_is_ignored`, `escape_when_closed_is_ignored`, `open_close_input_reset_the_highlight`, `input_reopens_and_clears_highlight`.
- [ ] **Step 4:** `CARGO_BUILD_JOBS=2 cargo test -p gus-combobox` → compile errors (items missing).
- [ ] **Step 5:** implement:

```rust
// matching.rs
pub fn matches<'a, T>(options: impl IntoIterator<Item = &'a T>, query: &str, label: impl Fn(&T) -> &str) -> Vec<&'a T>
where
    T: 'a,
{
    let query = query.trim().to_lowercase();
    let (mut starts, mut contains) = (Vec::new(), Vec::new());
    for option in options {
        let name = label(option).to_lowercase();
        if name.starts_with(&query) {
            starts.push(option);
        } else if name.contains(&query) {
            contains.push(option);
        }
    }
    starts.extend(contains);
    starts
}

pub fn completion<T>(matches: &[&T], query: &str, label: impl Fn(&T) -> &str) -> Option<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return None;
    }
    matches.iter().position(|option| label(option).to_lowercase().starts_with(&query))
}

// combobox.rs
impl Combobox {
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
            Nav::Escape if self.open && matches > 0 => {
                self.close();
                Outcome::Closed
            }
            Nav::Escape => Outcome::Ignored,
        }
    }
}
```
  with `open()` / `input()` → `open = true, highlighted = None`; `close()` → `open = false, highlighted = None`.
- [ ] **Step 6:** tests pass; fmt; clippy `-p gus-combobox`; commit `feat(gus-combobox): add matches, completion and list state`.

---

### Task 2: Controller on `gus-combobox`

**Files:** modify `crates/app-slint/Cargo.toml` (`gus-combobox = { path = "../gus-combobox" }`), `crates/app-slint/src/controller.rs`; test `crates/app-slint/tests/controller.rs` (append only).

**Produces:** `Controller::{tag_options, tag_suggestion, tag_focus, tag_input, tag_key, tag_pick, tag_highlighted, tag_list_open}`, `pub struct TagKey { pub handled: bool, pub text: String }`, re-export `pub use gus_combobox::Nav`.

- [ ] **Step 1:** failing tests (append): `tag_options_exclude_the_tasks_tags_and_rank_starts_with_first`, `tag_options_strip_hash_and_spaces`, `tag_options_are_capped_at_six`, `tag_key_down_then_enter_adds_the_highlighted_tag`, `tag_key_enter_without_highlight_adds_typed_text`, `enter_on_empty_query_adds_nothing`, `tag_key_tab_completes`, `tag_key_ignored_without_options`, `tag_key_escape_closes_then_is_ignored`, `tag_pick_adds_and_closes`. The existing `tag_suggestion` assertions (`tests/controller.rs:159-161`) stay as they are.
- [ ] **Step 2:** run → compile errors.
- [ ] **Step 3:** implement:

```rust
pub use gus_combobox::Nav;
use gus_combobox::{Combobox, Outcome};

const MAX_TAG_OPTIONS: usize = 6;

/// Resultado de uma tecla no campo de tag: `handled` = consumida; `text` = novo conteúdo do campo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagKey {
    pub handled: bool,
    pub text: String,
}

fn tag_query(query: &str) -> &str {
    query.trim().trim_start_matches('#')
}
```
  `tag_options`: project tags the task does not have, `gus_combobox::matches(.., tag_query(query), |t| t.name.as_str())`, `take(MAX_TAG_OPTIONS)`, names. `tag_suggestion`: `completion` over `tag_options`, name or `""`. `tag_key`: compute options + completion, `self.tag_combo.key(..)`, map outcomes per spec (Pick / UseText call `add_tag_by_name`, UseText skips an empty `tag_query`), returning `Result<TagKey, DomainError>`. `tag_pick(id, name)`: `add_tag_by_name` then `tag_combo.close()`.
- [ ] **Step 4:** controller tests pass (existing ones unchanged); fmt; clippy; commit `feat(app): rank tag suggestions with gus-combobox`.

---

### Task 3: Tag list in the UI

**Files:** modify `crates/app-slint/ui/{types.slint, globals.slint, task_row.slint, app.slint}`, `crates/app-slint/src/bind.rs`; create `crates/app-slint/tests/ui_tag_combobox.rs`; modify `CONTRIBUTING.md`.

- [ ] **Step 1:** failing UI test `tests/ui_tag_combobox.rs` (setup copied from `ui_tag_enter.rs`: project with tags `casa`, `carro`, `ui-kit`; select the project; ↓; `invoke_tag_selected`; three 20 ms waits):
  - after focus: `get_tag_list_open()` and `get_tag_options()` = `["carro", "casa", "ui-kit"]`, `get_tag_highlighted() == -1`;
  - type `ca` → options `["carro", "casa"]`; ↓ → highlighted 0; Enter → task tags `["carro"]`, list closed;
  - type `kit` → options `["ui-kit"]`; Esc → list closed, task still expanded; Esc → task collapsed;
  - re-focus, `invoke_tag_pick(id, "casa")` → tags `["carro", "casa"]`.
  - second test `escape_in_field_without_tags_reaches_the_app`: task in a project without tags; Ctrl+T; Esc → task collapsed.
- [ ] **Step 2:** run → compile error (no `get_tag_list_open`).
- [ ] **Step 3:** Slint:
  - `types.slint`: `export struct TagKey { handled: bool, text: string }`.
  - `globals.slint` (import `TagKey`): `in-out property <bool> tag-list-open; in-out property <[string]> tag-options; in-out property <int> tag-highlighted: -1; in-out property <int> tag-list-task: -1; in-out property <length> tag-anchor-x; in-out property <length> tag-anchor-y; in-out property <int> tag-clear-request;` and callbacks `tag-focus(int, bool, string)`, `tag-input(int, string)`, `tag-key(int, string, string, bool) -> TagKey`, `tag-pick(int, string)`.
  - `TagInput`: publish its window position (`changed` on `absolute-position`-based properties and on focus) to `tag-anchor-x/y`; `changed has-focus` → `tag-focus`; `edited` → `tag-input`; `key-pressed`: keep Backspace-on-empty, otherwise `tag-key(task, self.text, event.text, ctrl || alt || meta)` and apply `{handled, text}`; remove `accepted`; clear the field when `tag-clear-request` changes for this task.
  - `app.slint`: before the root's closing brace, `if Actions.tag-list-open && Actions.tag-options.length > 0 : TagList { window-height: root.height; }` — a `MenuPanel` with up to 6 rows of 26 px (`#name`, accent background on `tag-highlighted`, hover `Theme.chip`), click → `tag-pick`; placed under the anchor, or above it when it would leave the window.
- [ ] **Step 4:** `bind.rs`: `tag_nav(key, modified) -> Option<Nav>` (Return → Enter even with modifiers; ↓ ↑ Esc Tab only without modifiers); `sync_tag_list(s, id, query)` sets `tag-options` / `tag-highlighted` / `tag-list-open` / `tag-list-task`; wire the four callbacks; refresh rows after a tag is added; `log_err` on errors; bump `tag-clear-request` after `tag-pick`.
- [ ] **Step 5:** `git diff --stat -- crates/app-slint/tests/ui_*.rs` shows only the new file; `cargo test -p ray-task` passes.
- [ ] **Step 6:** CONTRIBUTING bullet: `- \`crates/gus-combobox\`: headless combobox (ranked matches, inline completion, list state), std only; drives the tag field's suggestion list.`
- [ ] **Step 7:** full verification (fmt, clippy, workspace tests, llvm-cov ≥ 93%, deny); commit `feat(app): show a tag suggestion list in the tag field`.
- [ ] **Step 8:** manual check for the human: list opens on Ctrl+T, filters, ↑/↓/Enter, click, Esc twice, does not cover important parts of the row, looks right in light and dark.
