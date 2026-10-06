# gus-list — design

Date: 2026-10-05
Status: draft, awaiting review

## Goal

Extract ray-task's list logic (grouping, keyboard selection, keyed model sync) into
a headless, UI-agnostic Rust library — `gus-list` — in the spirit of TanStack: pure state
and logic, rendering left to whatever UI framework the app uses. It is the first crate of a
future "GusStack" family.

v1 is **dogfood-first**: it covers exactly what ray-task needs today and nothing more.
ray-task is the first and only consumer; generalisation comes after real use.

Success criteria:

- `crates/gus-list` has zero normal dependencies (std only).
- ray-task's `Controller::rows()`, `Controller::move_selection` and `bind::sync_rows` are
  built on `gus-list` / `gus-list-slint`.
- The existing ray-task tests pass **unchanged** — proof the refactor preserves behaviour.
- `cargo test --workspace` is green and the coverage floor (92% lines) still holds.
- Manual check in the running app: text filter, grouped Upcoming view, arrow-key navigation
  and the exit animation of completed/deleted rows all behave as before.

## Decisions (from brainstorming)

| Decision | Choice | Why |
|---|---|---|
| v1 success criterion | Dogfood ray-task first, generalise later | How TanStack itself started (extracted from Nozzle); avoids designing for imaginary use cases |
| Scope | group + flatten, selection by key, keyed diff + Slint adapter | What ray-task does by hand today; `filter` dropped after spec review (see below) |
| Name | `gus-list` (+ `gus-list-slint`) | It is a list, not a table; both names free on crates.io |
| Location | Inside the ray-task workspace, hard boundary | Fast iteration; `gusstack` GitHub name is taken, org decision deferred; later extraction via `git subtree split` |
| API shape | Pure functions + no state types | App keeps owning its state (no second source of truth next to `Controller`); trivial to test; a builder can be layered on top later |

Out of scope for v1: filtering, sorting, column definitions, bucket (hash) grouping, header-as-own-row
entries, multi-select / range select, wrap-around navigation, move operations in the diff,
publishing to crates.io, creating a GitHub org.

## Why there is no `filter` in v1

The brainstorm proposed `filter(items, key, keep, pinned)` where pinned keys (rows animating
out) bypass the predicate. Checking the code during spec review showed that is not what
ray-task does: `Store::view` uses the keep set only to let *completed* tasks stay in the view
(`crates/core/src/store.rs:402`), and the text query is then applied to every row, kept ones
included. Using `pinned` would change behaviour; without `pinned` the function is just
`Iterator::filter`. Either way it does not earn its place, so it waits for a real need.

## Architecture

```
crates/gus-list/           # std only — the boundary rule
  src/lib.rs               # re-exports
  src/group.rs             # group_runs
  src/select.rs            # step
  src/diff.rs              # diff, Plan, Op, DEFAULT_MAX_IN_PLACE
crates/gus-list-slint/     # deps: gus-list, slint
  src/lib.rs               # sync (applies a Plan to a VecModel)
```

Dependency direction: `app-slint → gus-list-slint → gus-list`, and `app-slint → gus-list`.
`gus-list` never depends on `ray-core`, `chrono` or `slint`; its only generic bounds are
`Hash`, `Eq`, `PartialEq`, `Clone`.

Both crates are `publish = false` in v1 and get `[[package]]` entries in `release-plz.toml`
with `changelog_update = false` and `git_tag_enable = false`, like `ray-core`.

## Components

### `group_runs`

```rust
pub fn group_runs<'a, T: 'a, G: PartialEq>(
    items: impl IntoIterator<Item = &'a T>,
    group: impl Fn(&T) -> G,
) -> impl Iterator<Item = (Option<G>, &'a T)>;
```

- Groups **consecutive runs**: yields `Some(g)` on the first item of each run, `None` on the
  rest. Input is expected to be already ordered by group key (ray-task sorts in the store).
  O(n), stable, no reordering, no allocation beyond the iterator.
- "Header carried by the first row" matches ray-task's `Row.group_header` and keeps a single
  delegate type in the Slint list.
- Extra sections (ray-task's "Concluídas") are not a library concept: the app chains the
  two sequences and uses a group key that distinguishes them.

### `step`

```rust
pub fn step<'a, K: PartialEq>(
    keys: &'a [K],                // visual order
    current: Option<&K>,
    delta: isize,                 // ±1 arrows, ±N page, isize::MIN / isize::MAX = Home / End
    active: impl Fn(&K) -> bool,  // e.g. |k| !leaving.contains(k)
) -> Option<&'a K>;
```

Reproduces `Controller::move_selection` exactly:

- Works on the subsequence of active keys only.
- No active keys → `None`.
- `current` is `None`, inactive or absent → `delta >= 0` picks the first active key,
  `delta < 0` the last.
- Otherwise moves by `delta` with **clamping** (no wrap), using saturating arithmetic so
  `isize::MIN` / `isize::MAX` cannot overflow.

Not included: a "retain selection" helper. ray-task's `drop_stale_focus` deliberately uses
`in_view`, which ignores the text filter (editing a title must not close the task) — an app
rule; the generic version would be a one-line `Option::filter`.

Side benefit: `move_selection` stops calling `rows()` (which builds every `Row` with strings,
tones and tags) just to read ids; it builds only the key list.

### `diff`

```rust
pub enum Plan {
    Replace,                      // swap the whole model
    Patch(Vec<Op>),
}

pub enum Op {                     // indices are valid on the model *at the time of the op*
    Remove { at: usize },
    Insert { at: usize, from: usize },   // from = index into `new`
    Update { at: usize, from: usize },
}

pub const DEFAULT_MAX_IN_PLACE: usize = 64;

pub fn diff<K: Hash + Eq>(old: &[K], new: &[K], max_in_place: usize) -> Plan;
```

Same algorithm and heuristic as today's `sync_rows`, moved, not rewritten:

1. Index `old` by key; count reused keys.
2. `Replace` when fewer than half the rows are reused
   (`reused * 2 < max(old.len(), new.len())`) or when insertions + removals exceed
   `max_in_place`.
3. Otherwise a single greedy pass with a cursor over `old`: for each `new[i]`, if its old
   index `j >= cursor`, emit `Remove { at: i }` for each skipped old row, then
   `Update { at: i, from: i }` and set `cursor = j + 1`; else emit `Insert { at: i, from: i }`.
   Finally remove trailing rows.
4. A row that moved backwards is removed and re-inserted (no move op: it would need LIS and
   Slint's `VecModel` has no native move).

Precondition: keys are unique within `old` and within `new` (`debug_assert`).
`Op` carries only indices, so it fits any retained-mode UI, not just Slint.

### `gus_list_slint::sync`

```rust
pub fn sync<T, K>(model: &VecModel<T>, rows: Vec<T>, key: impl Fn(&T) -> K)
where
    T: Clone + PartialEq + 'static,
    K: Hash + Eq;
```

- Reads the old keys from the model, calls `diff(.., DEFAULT_MAX_IN_PLACE)`, applies the plan
  (`set_vec` for `Replace`; `remove` / `insert` / `set_row_data` for `Patch`).
- Improvement over today: `Update` calls `set_row_data` only when the row actually changed
  (`old != new`). Today every reused row notifies Slint, so the clock tick dirties the whole
  list.
- ray-task: `sync_rows(&model, rows)` becomes `gus_list_slint::sync(&model, rows, |r| r.id)`.

## Data flow in ray-task after the change

```
store.view(view, &keep)
  → .filter(|t| matches_query(t, &filter))  (unchanged)
  → gus_list::group_runs(.., |t| due date)   (Upcoming only)
  → chain "Concluídas" section                (project view with show_done)
  → Controller::row(..)                       → Vec<Row> → Vec<TaskItem>
  → gus_list_slint::sync(&tasks_model, items, |r| r.id)

move_selection:
  active keys from the same pipeline → gus_list::step(&keys, selected, delta, |k| !leaving)
```

## Error handling

No function returns `Result`: all are total. Empty input yields empty output or `None`.
The only precondition (unique keys in `diff`) is checked with `debug_assert`. No panics in
release builds.

## Testing

No extra dev-dependencies (keeps builds light on the maintainer's machine).

- `group_runs`, `step`: unit tests with edge cases — empty input, single group, all groups of size one, `isize::MIN` / `isize::MAX`, inactive
  `current`, all keys inactive.
- `diff`: exhaustive invariant test — for every pair `(old, new)` of ordered subsets of up to
  5 distinct keys, applying the `Patch` to a simulated `Vec<K>` yields exactly `new`;
  plus tests that pin the `Replace` thresholds.
- `gus-list-slint`: tests on a real `VecModel` (no window/backend needed) — replace path,
  patch path, unchanged rows are not re-set.
- Boundary: a test in `gus-list` reads its own `Cargo.toml` via `include_str!` and fails if
  it has a `[dependencies]` section.
- Doctests: one example per public function.
- ray-task: existing `Controller` / `bind` tests pass without modification.

Builds run with `CARGO_BUILD_JOBS=2`, one at a time.

## Migration order

1. Add `crates/gus-list` with `group_runs`, `step`, `diff` and their tests.
2. Use `group_runs` inside `Controller::rows()`.
3. Rewrite `Controller::move_selection` on `step`, building only keys.
4. Add `crates/gus-list-slint` with `sync` and its tests; replace `bind::sync_rows`.
5. Register both crates in the workspace and `release-plz.toml`; run the full test suite and
   the manual app check.
