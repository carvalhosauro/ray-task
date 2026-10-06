# gus-anim-state — design

Date: 2026-10-05
Status: draft, awaiting review

## Goal

Extract the timing of ray-task's row animations (a completed task lingers then leaves, a
deleted task leaves, a task whose time arrives pulses) into a headless, std-only crate —
`gus-anim-state` — the second GusStack crate after `gus-list`.

Today the timing is spread across three `HashSet`s in `Controller` (`lingering`, `leaving`,
`pulsing`) and four `Timer::single_shot` calls in `bind.rs` (600 ms, 220 ms, 220 ms, 700 ms),
and `start_leaving` must re-check whether a task is still done because undo can race the
timer. The crate replaces that with a deadline-based timeline the app advances with "now".

v1 is **dogfood-first**: it covers what ray-task needs, nothing more.

Success criteria:

- `crates/gus-anim-state` has zero dependencies (std only).
- `Controller` has no `lingering`, `leaving` or `pulsing` sets; `bind.rs` has no animation
  `Timer::single_shot` and drives animations through a single timer.
- Every `crates/app-slint/tests/ui_*.rs` test passes **unchanged** — the proof that behaviour
  is preserved.
- `cargo test --workspace` green, line coverage ≥ 92%, clippy / fmt / deny clean.
- Manual check in the running app: complete, delete, undo during the animation, and the
  minute pulse look as before.

## Decisions (from brainstorming)

| Decision | Choice | Why |
|---|---|---|
| Who owns time | The crate stores deadlines; the app passes `now` | Removes scattered timers and the undo race; testable with fixed `Duration`s |
| State shape | One generic `Timeline<K, P>`: each key plays a script of `(phase, duration)` steps | One primitive covers exit (linger → leave), delete and pulse; the app's phase enum keeps call sites readable |
| Time type | `Duration` since an app-chosen origin | `Instant` cannot be built at arbitrary values in tests |
| Slint glue | A small monotonic clock + one timer inside `bind.rs`, no adapter crate | `Controller` must query phases without Slint; extract an adapter only when a second app needs it |
| `fresh` (first-render grow) | Out of scope | Not time-based: cleared right after the first render |

Out of scope: easing / 0..1 progress (Slint animates), pause/resume, looping scripts,
callbacks inside the crate, an adapter crate, publishing, a GitHub org.

## Architecture

```
crates/gus-anim-state/          # std only
  src/lib.rs                    # crate docs + re-exports
  src/timeline.rs               # Timeline, Change
  tests/boundary.rs             # no-dependencies rule (same detector as gus-list)
```

Dependency direction: `app-slint → gus-anim-state`. Same rules as `gus-list`: no reference to
`ray-core`, `chrono`, `slint`; `publish = false`; a `[[package]]` entry in `release-plz.toml`
with `changelog_update = false` and `git_tag_enable = false`.

## `Timeline<K, P>`

```rust
pub struct Change<K, P> {
    pub key: K,
    pub from: P,
    pub to: Option<P>,            // None = the script finished; the key is gone
}

impl<K: Hash + Eq + Clone, P: Clone> Timeline<K, P> {
    pub fn new() -> Self;                                                // also Default
    pub fn play(&mut self, key: K, script: &[(P, Duration)], now: Duration);
    pub fn cancel(&mut self, key: &K) -> Option<P>;
    pub fn advance(&mut self, now: Duration) -> Vec<Change<K, P>>;
    pub fn phase(&self, key: &K) -> Option<&P>;
    pub fn keys(&self) -> impl Iterator<Item = &K>;
    pub fn next_deadline(&self) -> Option<Duration>;
}
```

Behaviour:

- `play` starts the script at its first phase immediately; that phase ends at
  `now + duration`, the next one starts there, and so on. After the last phase the key is
  removed.
- `play` on a key that is already playing **replaces** its script (e.g. delete during the
  linger goes straight to the delete script). No `Change` is emitted for the replaced phase.
- `play` with an empty script is the same as `cancel`.
- `cancel` removes the key at once and returns its phase, without emitting a `Change`;
  unknown key → `None`.
- `advance(now)` applies every step whose deadline is `<= now`, **all of them**, so a long
  suspension still yields every intermediate `Change` in order (e.g. `Lingering → Leaving`
  then `Leaving → None`). Across keys, changes are ordered by deadline, ties broken by the
  order of the `play` calls (an internal sequence number — `HashMap` order is not stable).
- A zero-duration phase is not skipped: its `Change` comes out of the next `advance`, never
  out of `play`, so the app handles every transition in one place.
- `advance` with a `now` earlier than a previous one does nothing (no deadline is due); it
  never panics.
- `next_deadline` is the earliest pending deadline, or `None` when nothing is playing. The app
  arms one timer for it after every `play`, `cancel` and `advance`.

No function returns `Result` or panics. Implementation: a `HashMap<K, Track<P>>`; `advance`
repeatedly takes the earliest due track and steps it — O(n·k) for n keys and k steps, which
is negligible for dozens of keys and two-step scripts.

## ray-task integration

### Clock and timer (`bind.rs`)

Slint's test backend mocks time (`mock_elapsed_time`) through a platform clock the app cannot
read, so a plain `Instant::now()` would fall out of step with the UI tests. `bind` keeps a
monotonic clock:

```rust
struct AnimClock { start: Instant, last: Cell<Duration> }
// now() = max(start.elapsed(), last)
// when the timer armed for deadline D fires: last = max(last, D)
```

When the single-shot timer fires, deadline `D` has passed by definition — in real or mocked
time — so the clock never lags it. In production real time is already `>= D`; in tests the
clock jumps to `D` when the mocked timer fires, and a following `play` schedules from there.

One `anim_timer: Timer` replaces the 600 / 220 / 220 / 700 ms single-shots. After each
`play` / `cancel` / `advance` the binding re-arms it for `next_deadline() - now` (or stops it
on `None`). When it fires: `last = max(last, D)`, `Controller::advance(now)`, refresh, show the
"Tarefa apagada" toast if a task was deleted, re-arm.

### Controller

```rust
enum Exit { Lingering, Leaving, Deleting }   // Deleting: delete from the store when it ends
```

- `lingering` / `leaving` / `pulsing` become `exits: Timeline<TaskId, Exit>` and
  `pulses: Timeline<TaskId, ()>` (two instances: a pulse may overlap an exit on the same key).
- `keep()` becomes the keys of `exits`; `Row.leaving` is `matches!(exits.phase(id), Some(Leaving | Deleting))`;
  `Row.pulse` is `pulses.phase(id).is_some()`; the `step` active predicate uses the same
  leaving test.
- `toggle(id, now)`: completed → `play(id, [(Lingering, 600ms), (Leaving, 220ms)])`;
  un-completed → `cancel(id)`.
- `begin_delete(id, now)` → `play(id, [(Deleting, 220ms)])`.
- `tick(previous, now)` → `pulses.play(id, [((), 700ms)])` for each task whose time arrived;
  still returns those ids.
- `undo()` cancels exits whose task is no longer done (today's stale-lingering cleanup, now
  through `cancel`).
- New `advance(now) -> Vec<TaskId>`: advances both timelines; `Leaving → None` does what
  `finish_leaving` does today (drop `expanded` / `selected` if they point at the task);
  `Deleting → None` deletes the task from the store, records it in `deleted` and flushes.
  Returns the ids actually deleted (for the toast).
- Removed: `start_leaving`, `finish_leaving`, `finish_delete`, `clear_pulse`.

### Tests that change

Unlike `gus-list`, the `Controller` API changes, so some existing controller tests change:

- The 8 tests in `crates/app-slint/tests/controller.rs` that call removed methods
  (`completed_task_lingers_then_leaves`, `delete_happens_only_after_the_animation`,
  `lingering_task_stays_out_of_completed_section`, `project_view_can_show_completed_section`,
  `tick_pulses_only_tasks_that_just_became_due`, `undo_after_leaving_started_clears_animation_state`,
  `undo_during_linger_keeps_the_task`, `undoing_a_delete_selects_the_restored_task`) are
  rewritten to drive time with `advance(now)`, **keeping their assertions**.
- Other call sites of `toggle` / `begin_delete` / `tick` get the extra `now` argument —
  a mechanical change.
- `crates/app-slint/tests/ui_*.rs` do not change. They exercise complete, delete, undo and
  pulse through the real UI with mocked time and are the behaviour proof.

## Error handling

All `Timeline` functions are total; no `Result`, no panics in release. `Controller::advance`
logs a failed store delete the same way `finish_delete` does today (`log_err` in `bind`), and
does not report that id as deleted.

## Testing

- `gus-anim-state` unit tests with fixed `Duration`s: full script; replacement on `play`;
  empty script; `cancel` known/unknown; long suspension emits every step in order; ordering
  across keys by deadline then `play` order; zero-duration phase; time going backwards;
  `next_deadline` before/after; `phase` / `keys` views.
- Boundary test identical to `gus-list`'s (detects any `*dependencies*` table).
- New controller tests: delete during the linger goes straight to `Deleting`; un-completing
  during `Leaving` cancels; one late `advance` finishes both exit steps in order; the pulse
  clears after 700 ms; `advance` returns deleted ids.
- No extra dev-dependencies. Builds with `CARGO_BUILD_JOBS=2`, one at a time.

## Migration order

1. Add `crates/gus-anim-state` with `Timeline` and its tests.
2. Move `Controller` to `exits` / `pulses` + `advance(now)`; rewrite the 8 controller tests and
   update call sites; add the new controller tests.
3. Replace the animation timers in `bind.rs` with `AnimClock` + one `anim_timer`; run the UI
   tests unchanged, the full suite, coverage, deny and the manual check.
