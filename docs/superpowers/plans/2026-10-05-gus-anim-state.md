# gus-anim-state Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the std-only `gus-anim-state` crate (`Timeline<K, P>`) and move ray-task's row-animation timing (linger → leave, delete, pulse) onto it, driven by one timer in `bind.rs`.

**Architecture:** `Timeline` stores per-key scripts of `(phase, duration)` steps with absolute deadlines; the app calls `advance(now)` and reacts to the returned `Change`s. `Controller` owns two timelines (`exits`, `pulses`) and exposes `advance(now)` / `next_anim_deadline()`; `bind.rs` owns a monotonic `AnimClock` and one `anim_timer` armed for the next deadline.

**Tech Stack:** Rust 2021 (MSRV 1.92), Slint 1.18 (`i-slint-backend-testing` for UI tests), cargo workspace, lefthook, cargo-deny, cargo-llvm-cov.

**Spec:** `docs/superpowers/specs/2026-10-05-gus-anim-state-design.md`

## Global Constraints

- `crates/gus-anim-state` has **no dependency tables at all** (no `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, `[target.*.dependencies]`).
- `gus-anim-state` must not reference `ray-core`, `chrono`, `slint` or any ray-task type.
- New crate: `edition.workspace`, `version.workspace`, `rust-version.workspace` (1.92), `license.workspace` (MIT), `repository.workspace`, `publish = false`; `[[package]]` entry in `release-plz.toml` with `changelog_update = false`, `git_tag_enable = false`.
- No `gus-anim-state` function returns `Result` or panics in release builds; time arithmetic saturates.
- Animation durations stay exactly: linger **600 ms**, leave **220 ms**, delete **220 ms**, pulse **700 ms**.
- Every `crates/app-slint/tests/ui_*.rs` file must pass **without modification**.
- In `crates/app-slint/tests/controller.rs`, only the 8 tests named in the spec plus `arrow_selection_skips_leaving_rows` (extra `now` argument) change, and they keep their assertions; new tests may be added.
- Style: `rustfmt.toml` (max_width 140, `use_small_heuristics = "Max"`); ray-task comments in Portuguese, `gus-anim-state` docs in English.
- CI gates: fmt, `clippy --workspace --all-targets -D warnings`, `cargo test --workspace`, MSRV 1.92 check, line coverage ≥ 92%, `cargo deny check`.
- Commit subjects: Conventional Commits (`type(scope): summary`).
- Maintainer's machine freezes under load: `CARGO_BUILD_JOBS=2`, one cargo command at a time, `-p` while iterating.

## Review Focus

1. **Undo while another task is being deleted** — undo's "task no longer done → cancel its exit" cleanup must not cancel a `Deleting` script (the task being deleted is usually not done). Expected: the delete still happens. Test: `undo_does_not_cancel_a_pending_delete` (Task 2).
2. **A new animation that ends before the one the timer is armed for** — the timer must be re-armed to the earlier deadline, not wait for the later one. Expected: `next_anim_deadline` is the minimum across both timelines. Test: `next_anim_deadline_is_the_earliest` (Task 2).
3. **Absurd durations or times (`Duration::MAX`)** — no overflow panic; deadlines saturate. Test: `huge_durations_saturate` (Task 1).
4. **The task disappears by other means while its delete animation runs** (its project deleted in the 220 ms) — no panic, the failure is reported, no "Tarefa apagada" toast for it. Test: `failed_delete_is_reported` (Task 2).
5. **Complete, un-complete, complete again quickly** — the second completion gets a full fresh 600 ms linger. Test: `recompleting_restarts_the_linger` (Task 2).

## Known behaviour difference (intended)

Item 5 differs from today: the old code's first 600 ms timer still fires after a quick
un-complete/re-complete, sees the task done and starts leaving early. With `Timeline`, `play`
replaces the script, so the linger restarts. This is the undo race the spec removes; the UI
tests do not exercise it.

## File Structure

```
Cargo.toml                                    # modify: workspace member
release-plz.toml                              # modify: [[package]] gus-anim-state
CONTRIBUTING.md                               # modify: architecture bullet
crates/gus-anim-state/Cargo.toml              # create
crates/gus-anim-state/src/lib.rs              # create: crate docs + re-exports
crates/gus-anim-state/src/timeline.rs         # create: Timeline, Change + unit tests
crates/gus-anim-state/tests/boundary.rs       # create: no-dependencies rule
crates/app-slint/Cargo.toml                   # modify: depend on gus-anim-state
crates/app-slint/src/controller.rs            # modify: exits/pulses timelines, advance, next_anim_deadline
crates/app-slint/src/bind.rs                  # modify: AnimClock, anim_timer, arm_anim, anim_fired
crates/app-slint/tests/controller.rs          # modify: rewrite 8 tests, now args, new tests
```

---

### Task 1: `gus-anim-state` crate with `Timeline`

**Files:**
- Create: `crates/gus-anim-state/Cargo.toml`, `src/lib.rs`, `src/timeline.rs`, `tests/boundary.rs`
- Modify: `Cargo.toml` (members), `release-plz.toml`

**Interfaces:**
- Consumes: nothing.
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct Change<K, P> { pub key: K, pub from: P, pub to: Option<P> }
  pub struct Timeline<K, P>;                                   // impl Default
  impl<K: Hash + Eq + Clone, P: Clone> Timeline<K, P> {
      pub fn new() -> Self;
      pub fn play(&mut self, key: K, script: &[(P, Duration)], now: Duration);
      pub fn cancel(&mut self, key: &K) -> Option<P>;
      pub fn advance(&mut self, now: Duration) -> Vec<Change<K, P>>;
      pub fn phase(&self, key: &K) -> Option<&P>;
      pub fn keys(&self) -> impl Iterator<Item = &K> + '_;
      pub fn next_deadline(&self) -> Option<Duration>;
  }
  ```

- [ ] **Step 1: Create the manifest and register the crate**

`crates/gus-anim-state/Cargo.toml`:

```toml
[package]
name = "gus-anim-state"
edition.workspace = true
version.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false
repository.workspace = true
description = "Headless animation timing: per-key scripts of timed phases, advanced by the app."
```

Root `Cargo.toml`:

```toml
members = ["crates/core", "crates/gus-list", "crates/gus-list-slint", "crates/gus-anim-state", "crates/app-slint"]
```

Append to `release-plz.toml`:

```toml

[[package]]
name = "gus-anim-state"
changelog_update = false
git_tag_enable = false
```

- [ ] **Step 2: Write the boundary test**

`crates/gus-anim-state/tests/boundary.rs`:

```rust
//! gus-anim-state is std only: no dependencies of any kind.

/// Every TOML table header in `manifest` that declares dependencies of any kind.
fn dependency_tables(manifest: &str) -> Vec<&str> {
    manifest.lines().map(str::trim).filter(|line| line.starts_with('[') && line.contains("dependencies")).collect()
}

#[test]
fn manifest_has_no_dependencies() {
    assert_eq!(
        dependency_tables(include_str!("../Cargo.toml")),
        Vec::<&str>::new(),
        "gus-anim-state must stay std-only, dev-dependencies included"
    );
}

#[test]
fn detector_catches_every_dependency_table_form() {
    let tables =
        ["[dependencies]", "[dev-dependencies]", "[build-dependencies]", "[target.'cfg(unix)'.dependencies]", "[dependencies.foo]"];
    for table in tables {
        assert_eq!(dependency_tables(table), vec![table], "missed {table}");
    }
}
```

- [ ] **Step 3: Write the failing `Timeline` tests**

`crates/gus-anim-state/src/timeline.rs`:

```rust
//! Per-key scripts of timed phases.

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Change, Timeline};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Exit {
        Lingering,
        Leaving,
    }
    use Exit::{Leaving, Lingering};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    const EXIT: [(Exit, Duration); 2] = [(Lingering, Duration::from_millis(600)), (Leaving, Duration::from_millis(220))];

    fn change(key: u32, from: Exit, to: Option<Exit>) -> Change<u32, Exit> {
        Change { key, from, to }
    }

    #[test]
    fn empty_timeline_has_nothing() {
        let mut t: Timeline<u32, Exit> = Timeline::new();
        assert_eq!(t.phase(&1), None);
        assert_eq!(t.keys().count(), 0);
        assert_eq!(t.next_deadline(), None);
        assert!(t.advance(ms(1_000)).is_empty());
    }

    #[test]
    fn play_starts_the_first_phase_immediately_without_a_change() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(100));
        assert_eq!(t.phase(&1), Some(&Lingering));
        assert_eq!(t.keys().copied().collect::<Vec<_>>(), vec![1]);
        assert_eq!(t.next_deadline(), Some(ms(700)));
    }

    #[test]
    fn script_runs_step_by_step_then_the_key_is_gone() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        assert!(t.advance(ms(599)).is_empty());
        assert_eq!(t.advance(ms(600)), vec![change(1, Lingering, Some(Leaving))]);
        assert_eq!(t.phase(&1), Some(&Leaving));
        assert_eq!(t.next_deadline(), Some(ms(820)));
        assert_eq!(t.advance(ms(820)), vec![change(1, Leaving, None)]);
        assert_eq!(t.phase(&1), None);
        assert_eq!(t.next_deadline(), None);
    }

    #[test]
    fn long_suspension_emits_every_step_in_order() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        assert_eq!(t.advance(ms(10_000)), vec![change(1, Lingering, Some(Leaving)), change(1, Leaving, None)]);
    }

    #[test]
    fn changes_across_keys_follow_deadlines_then_play_order() {
        let mut t = Timeline::new();
        t.play(2, &[(Leaving, ms(300))], ms(0));
        t.play(1, &EXIT, ms(0));
        t.play(3, &[(Leaving, ms(300))], ms(0));
        assert_eq!(
            t.advance(ms(1_000)),
            vec![change(2, Leaving, None), change(3, Leaving, None), change(1, Lingering, Some(Leaving)), change(1, Leaving, None)]
        );
    }

    #[test]
    fn play_on_a_playing_key_replaces_its_script() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.play(1, &[(Leaving, ms(220))], ms(100));
        assert_eq!(t.phase(&1), Some(&Leaving));
        assert_eq!(t.next_deadline(), Some(ms(320)));
        assert_eq!(t.advance(ms(1_000)), vec![change(1, Leaving, None)]);
    }

    #[test]
    fn replaying_restarts_the_clock() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.play(1, &EXIT, ms(400));
        assert!(t.advance(ms(999)).is_empty());
        assert_eq!(t.advance(ms(1_000)), vec![change(1, Lingering, Some(Leaving))]);
    }

    #[test]
    fn empty_script_cancels() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.play(1, &[], ms(100));
        assert_eq!(t.phase(&1), None);
        assert!(t.advance(ms(1_000)).is_empty());
    }

    #[test]
    fn cancel_removes_at_once_and_returns_the_phase() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.advance(ms(600));
        assert_eq!(t.cancel(&1), Some(Leaving));
        assert_eq!(t.phase(&1), None);
        assert_eq!(t.next_deadline(), None);
        assert!(t.advance(ms(1_000)).is_empty());
        assert_eq!(t.cancel(&1), None);
        assert_eq!(t.cancel(&99), None);
    }

    #[test]
    fn zero_duration_phase_changes_on_the_next_advance() {
        let mut t = Timeline::new();
        t.play(1, &[(Leaving, ms(0))], ms(50));
        assert_eq!(t.phase(&1), Some(&Leaving));
        assert_eq!(t.next_deadline(), Some(ms(50)));
        assert_eq!(t.advance(ms(50)), vec![change(1, Leaving, None)]);
    }

    #[test]
    fn time_going_backwards_does_nothing() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(1_000));
        assert!(t.advance(ms(0)).is_empty());
        assert_eq!(t.phase(&1), Some(&Lingering));
    }

    #[test]
    fn huge_durations_saturate() {
        let mut t = Timeline::new();
        t.play(1, &[(Lingering, Duration::MAX), (Leaving, Duration::MAX)], ms(5));
        assert_eq!(t.next_deadline(), Some(Duration::MAX));
        assert_eq!(t.advance(Duration::MAX), vec![change(1, Lingering, Some(Leaving)), change(1, Leaving, None)]);
    }

    #[test]
    fn independent_keys_keep_their_own_scripts() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.play(2, &EXIT, ms(500));
        assert_eq!(t.advance(ms(600)), vec![change(1, Lingering, Some(Leaving))]);
        assert_eq!(t.phase(&2), Some(&Lingering));
        assert_eq!(t.next_deadline(), Some(ms(820)));
    }
}
```

`crates/gus-anim-state/src/lib.rs`:

```rust
//! Headless animation timing for retained-mode UIs.
//!
//! Each key plays a script of timed phases; the app passes "now" and reacts to the phase
//! changes [`Timeline::advance`] returns. No clock, no threads, no callbacks: easy to drive from
//! any event loop and to test with fixed times. Part of the GusStack family; extracted from
//! ray-task.

mod timeline;

pub use timeline::{Change, Timeline};
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `CARGO_BUILD_JOBS=2 cargo test -p gus-anim-state`
Expected: compile errors `unresolved imports `timeline::Change`, `timeline::Timeline``.

- [ ] **Step 5: Implement `Timeline`**

Insert above the test module in `crates/gus-anim-state/src/timeline.rs`:

```rust
use std::collections::HashMap;
use std::hash::Hash;
use std::time::Duration;

/// A key moved from one phase to the next (`to: Some`) or finished its script (`to: None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change<K, P> {
    pub key: K,
    pub from: P,
    pub to: Option<P>,
}

struct Track<P> {
    steps: Vec<(P, Duration)>,
    index: usize,
    ends_at: Duration,
    seq: u64,
}

/// Keys playing scripts of `(phase, duration)` steps.
///
/// Time is a [`Duration`] since an origin the app chooses; pass the same clock to every call.
///
/// ```
/// use std::time::Duration;
/// use gus_anim_state::{Change, Timeline};
///
/// let ms = Duration::from_millis;
/// let mut t = Timeline::new();
/// t.play("row", &[("lingering", ms(600)), ("leaving", ms(220))], ms(0));
/// assert_eq!(t.phase(&"row"), Some(&"lingering"));
/// assert_eq!(t.next_deadline(), Some(ms(600)));
/// assert_eq!(t.advance(ms(600)), vec![Change { key: "row", from: "lingering", to: Some("leaving") }]);
/// assert_eq!(t.advance(ms(820)), vec![Change { key: "row", from: "leaving", to: None }]);
/// ```
pub struct Timeline<K, P> {
    tracks: HashMap<K, Track<P>>,
    next_seq: u64,
}

impl<K, P> Default for Timeline<K, P> {
    fn default() -> Self {
        Self { tracks: HashMap::new(), next_seq: 0 }
    }
}

impl<K: Hash + Eq + Clone, P: Clone> Timeline<K, P> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts `script` for `key` at `now`, replacing whatever the key was playing. The first
    /// phase is current immediately (no [`Change`] for it). An empty script is a [`cancel`](Self::cancel).
    pub fn play(&mut self, key: K, script: &[(P, Duration)], now: Duration) {
        let Some((_, first)) = script.first() else {
            self.tracks.remove(&key);
            return;
        };
        let seq = self.next_seq;
        self.next_seq += 1;
        self.tracks.insert(key, Track { steps: script.to_vec(), index: 0, ends_at: now.saturating_add(*first), seq });
    }

    /// Stops `key` at once, without a [`Change`]. Returns the phase it was in.
    pub fn cancel(&mut self, key: &K) -> Option<P> {
        self.tracks.remove(key).map(|mut track| track.steps.swap_remove(track.index).0)
    }

    /// Applies every step whose deadline is `<= now`, earliest first (ties: the key played
    /// first). A long gap yields every intermediate change, in order.
    pub fn advance(&mut self, now: Duration) -> Vec<Change<K, P>> {
        let mut changes = Vec::new();
        loop {
            let due = self.tracks.iter().filter(|(_, t)| t.ends_at <= now).min_by_key(|(_, t)| (t.ends_at, t.seq)).map(|(k, _)| k.clone());
            let Some(key) = due else { break };
            let Some(track) = self.tracks.get_mut(&key) else { break };
            let from = track.steps[track.index].0.clone();
            track.index += 1;
            match track.steps.get(track.index) {
                Some((phase, duration)) => {
                    let to = phase.clone();
                    track.ends_at = track.ends_at.saturating_add(*duration);
                    changes.push(Change { key, from, to: Some(to) });
                }
                None => {
                    self.tracks.remove(&key);
                    changes.push(Change { key, from, to: None });
                }
            }
        }
        changes
    }

    /// The phase `key` is in, if it is playing.
    pub fn phase(&self, key: &K) -> Option<&P> {
        self.tracks.get(key).map(|track| &track.steps[track.index].0)
    }

    /// Every key currently playing, in no particular order.
    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.tracks.keys()
    }

    /// The earliest pending deadline: arm one timer for it after each `play` / `cancel` / `advance`.
    pub fn next_deadline(&self) -> Option<Duration> {
        self.tracks.values().map(|track| track.ends_at).min()
    }
}
```

Note on `huge_durations_saturate`: both steps end at `Duration::MAX` (saturated), so one `advance(Duration::MAX)` applies both, in order.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p gus-anim-state`
Expected: PASS — 13 unit tests, 2 boundary tests, 1 doctest.

- [ ] **Step 7: Lint and commit**

Run: `cargo fmt --all && CARGO_BUILD_JOBS=2 cargo clippy -p gus-anim-state --all-targets -- -D warnings`
Expected: no warnings.

```bash
git add Cargo.toml Cargo.lock release-plz.toml crates/gus-anim-state
git commit -m "feat(gus-anim-state): add Timeline"
```

---

### Task 2: ray-task animations on `Timeline`

`Controller` and `bind.rs` change together: the controller API the binding calls changes, so they cannot be committed apart and still compile.

**Files:**
- Modify: `crates/app-slint/Cargo.toml`
- Modify: `crates/app-slint/src/controller.rs` (imports; `Controller` fields and `new`; `in_view`; `keep`; `row`; `move_selection`; `toggle`; `start_leaving` / `finish_leaving` / `begin_delete` / `finish_delete`; `undo`; `tick` / `clear_pulse`)
- Modify: `crates/app-slint/src/bind.rs` (imports; `Shared`; `bind`; `toggle`; `delete`; `minute_tick`; new `AnimClock`, `arm_anim`, `anim_fired`)
- Modify: `crates/app-slint/tests/controller.rs`
- Modify: `CONTRIBUTING.md`

**Interfaces:**
- Consumes: `gus_anim_state::{Timeline, Change}` (Task 1).
- Produces (Controller public API after this task):
  ```rust
  pub fn toggle(&mut self, id: TaskId, now: Duration) -> Result<bool, DomainError>;
  pub fn begin_delete(&mut self, id: TaskId, now: Duration) -> bool;
  pub fn tick(&mut self, previous: NaiveDateTime, anim_now: Duration) -> Vec<TaskId>;
  pub fn advance(&mut self, now: Duration) -> Vec<Result<TaskId, DomainError>>;   // Ok = deleted, Err = delete failed
  pub fn next_anim_deadline(&self) -> Option<Duration>;
  // removed: start_leaving, finish_leaving, finish_delete, clear_pulse
  ```
  Spec clarification: the spec says `advance` returns "the ids actually deleted" and that a failed
  delete is logged "the same way `finish_delete` does today (`log_err` in `bind`)". Returning
  `Vec<Result<TaskId, DomainError>>` does both: `bind` logs each `Err` with `log_err` and counts the
  `Ok`s for the toast.

- [ ] **Step 1: Update the existing controller tests to the new API (they will not compile yet)**

In `crates/app-slint/tests/controller.rs`, add `use std::time::Duration;` after `use std::rc::Rc;`, and after `fn d(...)` add:

```rust
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
```

Replace these test bodies (assertions kept; time now driven by `advance`):

```rust
#[test]
fn completed_task_lingers_then_leaves() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    assert_eq!(f.c.toggle(id, ms(0)), Ok(true));
    let row = &f.c.rows()[0];
    assert!(row.done && !row.leaving);
    assert_eq!(f.c.nav_rows()[0].count, 0);
    f.c.advance(ms(600));
    assert!(f.c.rows()[0].leaving);
    f.c.advance(ms(820));
    assert!(f.c.rows().is_empty());
}

#[test]
fn undo_during_linger_keeps_the_task() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    f.c.toggle(id, ms(0)).unwrap();
    assert!(f.c.undo());
    f.c.advance(ms(1_000));
    let rows = f.c.rows();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].done && !rows[0].leaving);
}

#[test]
fn delete_happens_only_after_the_animation() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    assert!(f.c.begin_delete(id, ms(0)));
    assert!(f.c.rows()[0].leaving);
    assert!(f.c.store.task(id).is_some());
    assert_eq!(f.c.advance(ms(220)), vec![Ok(id)]);
    assert!(f.c.rows().is_empty());
    assert!(f.c.undo());
    assert_eq!(f.c.rows().len(), 1);
}
```

In `tick_pulses_only_tasks_that_just_became_due`, replace

```rust
    assert_eq!(f.c.tick(previous), vec![a]);
    assert!(f.c.rows().iter().find(|r| r.id == a).unwrap().pulse);
    f.c.clear_pulse();
```

with

```rust
    assert_eq!(f.c.tick(previous, ms(0)), vec![a]);
    assert!(f.c.rows().iter().find(|r| r.id == a).unwrap().pulse);
    f.c.advance(ms(700));
```

and `assert!(f.c.tick(previous).is_empty(), ...)` with `assert!(f.c.tick(previous, ms(800)).is_empty(), "virada do dia não pulsa nada");`.

In `project_view_can_show_completed_section`, replace

```rust
    f.c.toggle(a).unwrap();
    f.c.start_leaving(a);
    f.c.finish_leaving(a);
```

with

```rust
    f.c.toggle(a, ms(0)).unwrap();
    f.c.advance(ms(820));
```

In `undo_after_leaving_started_clears_animation_state`, replace

```rust
    f.c.toggle(id).unwrap();
    assert!(f.c.start_leaving(id));
```

with

```rust
    f.c.toggle(id, ms(0)).unwrap();
    f.c.advance(ms(600));
    assert!(f.c.rows()[0].leaving);
```

In `undoing_a_delete_selects_the_restored_task`, replace

```rust
    assert!(f.c.begin_delete(b));
    f.c.finish_delete(b).unwrap();
```

with

```rust
    assert!(f.c.begin_delete(b, ms(0)));
    assert_eq!(f.c.advance(ms(220)), vec![Ok(b)]);
```

In `lingering_task_stays_out_of_completed_section`, replace

```rust
    f.c.toggle(c).unwrap();
    f.c.start_leaving(c);
    f.c.finish_leaving(c);
    f.c.toggle_show_done();
    f.c.toggle(a).unwrap();
```

with

```rust
    f.c.toggle(c, ms(0)).unwrap();
    f.c.advance(ms(820));
    f.c.toggle_show_done();
    f.c.toggle(a, ms(1_000)).unwrap();
```

In `arrow_selection_skips_leaving_rows`, change `f.c.begin_delete(b)` → `f.c.begin_delete(b, ms(0))` and `f.c.begin_delete(c)` → `f.c.begin_delete(c, ms(0))`.

- [ ] **Step 2: Add the new controller tests**

Append to `crates/app-slint/tests/controller.rs`:

```rust
#[test]
fn deleting_during_linger_goes_straight_to_delete() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    assert!(f.c.begin_delete(a, ms(100)));
    assert!(f.c.rows()[0].leaving, "apagar não espera o fim do linger");
    assert_eq!(f.c.advance(ms(320)), vec![Ok(a)]);
    assert!(f.c.store.task(a).is_none());
    assert!(f.c.advance(ms(5_000)).is_empty(), "o roteiro de conclusão foi substituído");
}

#[test]
fn uncompleting_during_leaving_cancels_the_exit() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.advance(ms(600));
    assert!(f.c.rows()[0].leaving);
    assert_eq!(f.c.toggle(a, ms(700)), Ok(false));
    assert!(!f.c.rows()[0].leaving && !f.c.rows()[0].done);
    f.c.advance(ms(5_000));
    assert_eq!(f.c.rows().len(), 1);
    assert_eq!(f.c.next_anim_deadline(), None);
}

#[test]
fn recompleting_restarts_the_linger() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.toggle(a, ms(300)).unwrap();
    f.c.toggle(a, ms(400)).unwrap();
    f.c.advance(ms(900));
    assert!(!f.c.rows()[0].leaving, "linger novo de 600 ms a partir de 400");
    f.c.advance(ms(1_000));
    assert!(f.c.rows()[0].leaving);
}

#[test]
fn late_advance_finishes_every_step() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.begin_delete(b, ms(0));
    assert_eq!(f.c.advance(ms(60_000)), vec![Ok(b)]);
    assert!(f.c.rows().is_empty());
    assert_eq!(f.c.next_anim_deadline(), None);
}

#[test]
fn undo_does_not_cancel_a_pending_delete() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.begin_delete(b, ms(0));
    assert!(f.c.undo(), "desfaz a conclusão de a");
    assert_eq!(f.c.advance(ms(220)), vec![Ok(b)], "b continua sendo apagada");
    assert_eq!(f.c.rows().iter().map(|r| r.id).collect::<Vec<_>>(), vec![a]);
}

#[test]
fn next_anim_deadline_is_the_earliest() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    assert_eq!(f.c.next_anim_deadline(), None);
    f.c.toggle(a, ms(0)).unwrap();
    assert_eq!(f.c.next_anim_deadline(), Some(ms(600)));
    f.c.begin_delete(b, ms(100));
    assert_eq!(f.c.next_anim_deadline(), Some(ms(320)));
    f.c.advance(ms(320));
    assert_eq!(f.c.next_anim_deadline(), Some(ms(600)));
    f.c.advance(ms(600));
    assert_eq!(f.c.next_anim_deadline(), Some(ms(820)));
}

#[test]
fn failed_delete_is_reported() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.begin_delete(a, ms(0));
    f.c.store.delete_task(a).unwrap();
    let results = f.c.advance(ms(220));
    assert!(matches!(results.as_slice(), [Err(_)]), "{results:?}");
    assert!(f.c.undo(), "desfaz a remoção feita direto no store");
    assert_eq!(f.c.selected, None, "não foi o app que apagou: o desfazer não seleciona a tarefa");
}
```

- [ ] **Step 3: Run the controller tests to verify they fail**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test controller`
Expected: compile errors — `toggle` takes 1 argument, no method `advance` / `next_anim_deadline` on `Controller`.

- [ ] **Step 4: Add the dependency and move `Controller` to timelines**

`crates/app-slint/Cargo.toml`, after `gus-list-slint`:

```toml
gus-anim-state = { path = "../gus-anim-state" }
```

In `crates/app-slint/src/controller.rs`:

Imports — add `use std::time::Duration;` after `use std::collections::HashSet;` and `use gus_anim_state::Timeline;` after the `chrono` import.

After the `Section` enum, add:

```rust
/// Saída animada de uma linha. `Deleting` apaga a tarefa do store quando termina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exit {
    Lingering,
    Leaving,
    Deleting,
}

/// Concluída: fica visível um instante, depois sai.
const COMPLETE_SCRIPT: [(Exit, Duration); 2] = [(Exit::Lingering, Duration::from_millis(600)), (Exit::Leaving, Duration::from_millis(220))];
const DELETE_SCRIPT: [(Exit, Duration); 1] = [(Exit::Deleting, Duration::from_millis(220))];
/// A hora da tarefa chegou: a linha pulsa uma vez.
const PULSE_SCRIPT: [((), Duration); 1] = [((), Duration::from_millis(700))];
```

In `struct Controller`, replace

```rust
    lingering: HashSet<TaskId>,
    leaving: HashSet<TaskId>,
    pulsing: HashSet<TaskId>,
```

with

```rust
    exits: Timeline<TaskId, Exit>,
    pulses: Timeline<TaskId, ()>,
```

and in `Controller::new`, replace the three `HashSet::new()` lines for those fields with

```rust
            exits: Timeline::new(),
            pulses: Timeline::new(),
```

In `in_view`, replace `if self.lingering.contains(&id) || self.leaving.contains(&id) {` with `if self.exits.phase(&id).is_some() {`.

Replace `keep`:

```rust
    fn keep(&self) -> HashSet<TaskId> {
        self.exits.keys().copied().collect()
    }

    fn is_leaving(&self, id: TaskId) -> bool {
        matches!(self.exits.phase(&id), Some(Exit::Leaving | Exit::Deleting))
    }
```

In `row`, replace

```rust
            leaving: self.leaving.contains(&task.id),
            pulse: self.pulsing.contains(&task.id),
```

with

```rust
            leaving: self.is_leaving(task.id),
            pulse: self.pulses.phase(&task.id).is_some(),
```

In `move_selection`, replace `|id| !self.leaving.contains(id)` with `|id| !self.is_leaving(*id)`.

Replace `toggle`, `start_leaving`, `finish_leaving`, `begin_delete`, `finish_delete` with:

```rust
    /// Retorna `true` se ficou concluída (a linha fica visível até o fim da animação de saída).
    pub fn toggle(&mut self, id: TaskId, now: Duration) -> Result<bool, DomainError> {
        let done = self.store.toggle_complete(id)?;
        if done {
            self.exits.play(id, &COMPLETE_SCRIPT, now);
        } else {
            self.exits.cancel(&id);
        }
        self.flush();
        Ok(done)
    }

    pub fn begin_delete(&mut self, id: TaskId, now: Duration) -> bool {
        if self.store.task(id).is_none() {
            return false;
        }
        self.exits.play(id, &DELETE_SCRIPT, now);
        true
    }

    /// Avança as animações até `now`. Cada item é uma tarefa que terminou de sair apagando:
    /// `Ok` se foi apagada (o desfazer a restaura), `Err` se o store recusou.
    pub fn advance(&mut self, now: Duration) -> Vec<Result<TaskId, DomainError>> {
        self.pulses.advance(now);
        let mut deleted = Vec::new();
        for change in self.exits.advance(now) {
            if change.to.is_some() {
                continue;
            }
            let id = change.key;
            if self.expanded == Some(id) {
                self.expanded = None;
            }
            if self.selected == Some(id) {
                self.selected = None;
            }
            if change.from == Exit::Deleting {
                deleted.push(self.store.delete_task(id).map(|()| {
                    self.deleted.insert(id);
                    id
                }));
            }
        }
        if !deleted.is_empty() {
            self.flush();
        }
        deleted
    }

    /// Próximo prazo de qualquer animação (o binding arma um único timer para ele).
    pub fn next_anim_deadline(&self) -> Option<Duration> {
        self.exits.next_deadline().into_iter().chain(self.pulses.next_deadline()).min()
    }
```

In `undo`, replace

```rust
            let stale: Vec<TaskId> =
                self.lingering.iter().copied().filter(|id| !self.store.task(*id).is_some_and(|t| t.is_done())).collect();
            for id in stale {
                self.lingering.remove(&id);
                self.leaving.remove(&id);
            }
```

with

```rust
            // Conclusão desfeita no meio da saída: a linha fica. Quem está sendo apagada segue.
            let stale: Vec<TaskId> = self
                .exits
                .keys()
                .copied()
                .filter(|id| self.exits.phase(id) != Some(&Exit::Deleting) && !self.store.task(*id).is_some_and(|t| t.is_done()))
                .collect();
            for id in stale {
                self.exits.cancel(&id);
            }
```

Replace `tick` and `clear_pulse` with:

```rust
    /// Chamado a cada minuto. Retorna as tarefas cuja hora chegou desde `previous` (elas pulsam).
    pub fn tick(&mut self, previous: NaiveDateTime, anim_now: Duration) -> Vec<TaskId> {
        let now = self.store.now();
        self.drop_stale_focus(); // a virada do dia tira tarefas de Próximos
        if previous.date() != now.date() {
            return Vec::new();
        }
        let due_now: Vec<TaskId> = self
            .store
            .view(View::Today, &HashSet::new())
            .into_iter()
            .filter(|t| t.due.is_some_and(|d| d.date == now.date() && d.time.is_some_and(|tm| tm > previous.time() && tm <= now.time())))
            .map(|t| t.id)
            .collect();
        for id in &due_now {
            self.pulses.play(*id, &PULSE_SCRIPT, anim_now);
        }
        due_now
    }
```

- [ ] **Step 5: Move `bind.rs` to one animation timer**

In `crates/app-slint/src/bind.rs`:

Imports: change `use std::time::Duration;` to `use std::time::{Duration, Instant};`.

Add after the `Pending` enum:

```rust
/// Relógio das animações. O backend de testes do Slint adianta o tempo dos timers, não o
/// `Instant`; por isso o relógio nunca fica atrás do último prazo que o timer cumpriu.
struct AnimClock {
    start: Instant,
    last: Cell<Duration>,
}

impl AnimClock {
    fn new() -> Self {
        Self { start: Instant::now(), last: Cell::new(Duration::ZERO) }
    }

    fn now(&self) -> Duration {
        let now = self.start.elapsed().max(self.last.get());
        self.last.set(now);
        now
    }

    fn reached(&self, deadline: Duration) {
        self.last.set(self.last.get().max(deadline));
    }
}
```

In `struct Shared`, after `toast_timer: Timer,` add:

```rust
    anim_clock: AnimClock,
    anim_timer: Timer,
```

In `bind`, in the `Shared { .. }` literal after `toast_timer: Timer::default(),` add:

```rust
        anim_clock: AnimClock::new(),
        anim_timer: Timer::default(),
```

Replace `toggle` and `delete` with:

```rust
pub(crate) fn toggle(s: &Rc<Shared>, id: TaskId) {
    let now = s.anim_clock.now();
    let done = log_err(s.ctrl.borrow_mut().toggle(id, now), "concluir tarefa");
    refresh(s);
    arm_anim(s);
    if done == Some(true) {
        show_toast(s, "Tarefa concluída", "Desfazer", TOAST_UNDO);
    }
}

pub(crate) fn delete(s: &Rc<Shared>, id: TaskId) {
    let now = s.anim_clock.now();
    if !s.ctrl.borrow_mut().begin_delete(id, now) {
        return;
    }
    refresh(s);
    arm_anim(s);
}

/// Arma o timer único das animações para o próximo prazo; sem nada tocando, para.
/// Um prazo cancelado depois (desfazer) só acorda o timer à toa: `advance` não faz nada.
fn arm_anim(s: &Rc<Shared>) {
    let Some(deadline) = s.ctrl.borrow().next_anim_deadline() else {
        s.anim_timer.stop();
        return;
    };
    let delay = deadline.saturating_sub(s.anim_clock.now());
    let weak = Rc::downgrade(s);
    s.anim_timer.start(TimerMode::SingleShot, delay, move || {
        if let Some(s) = weak.upgrade() {
            anim_fired(&s, deadline);
        }
    });
}

fn anim_fired(s: &Rc<Shared>, deadline: Duration) {
    s.anim_clock.reached(deadline);
    let now = s.anim_clock.now();
    let results = s.ctrl.borrow_mut().advance(now);
    refresh(s);
    if results.into_iter().filter_map(|r| log_err(r, "apagar tarefa")).count() > 0 {
        show_toast(s, "Tarefa apagada", "Desfazer", TOAST_UNDO);
    }
    arm_anim(s);
}
```

Why `Weak`: `anim_timer` lives inside `Shared`; a closure holding `Rc<Shared>` would keep `Shared` alive forever.

Replace the body of `minute_tick`:

```rust
fn minute_tick(s: &Rc<Shared>, last: &Cell<NaiveDateTime>) {
    let now = s.ctrl.borrow().store.now();
    let previous = last.replace(now);
    let anim_now = s.anim_clock.now();
    s.ctrl.borrow_mut().tick(previous, anim_now);
    refresh(s);
    arm_anim(s);
}
```

Afterwards, `grep -n "single_shot" crates/app-slint/src/bind.rs` must list only the non-animation ones (view crossfade 60 ms, the two 16 ms focus hops, and the minute-alignment in `start_clock`).

- [ ] **Step 6: Run the controller tests**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test controller`
Expected: PASS — all tests, including the 7 new ones.

- [ ] **Step 7: Run the UI tests unchanged**

Run: `git diff --stat -- crates/app-slint/tests/ui_*.rs` → empty.
Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task`
Expected: PASS — every `tests/ui_*.rs` (notably `ui_tasks`, `ui_keyboard`, `ui_toast`), `tests/controller.rs`, unit tests.

If a UI test fails because restarting `anim_timer` from inside its own callback does not fire, stop and debug (superpowers:systematic-debugging) before changing the design; the fallback is a `Timer::single_shot` per arm guarded by a generation counter in `Shared`, recorded as a ruling.

- [ ] **Step 8: Update the architecture notes**

In `CONTRIBUTING.md` ("Architecture at a glance"), after the `gus-list` bullet add:

```markdown
- `crates/gus-anim-state`: headless animation timing (`Timeline`: per-key scripts of timed phases), std only.
```

- [ ] **Step 9: Full verification, one command at a time**

```bash
cargo fmt --all -- --check
CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=2 cargo test --workspace
CARGO_BUILD_JOBS=2 cargo llvm-cov --workspace --summary-only
cargo deny check
```

Expected: fmt clean; no clippy warnings; all tests pass; TOTAL line coverage ≥ 92%; `advisories ok, bans ok, licenses ok, sources ok`.

- [ ] **Step 10: Manual check (for the human)**

`CARGO_BUILD_JOBS=2 cargo run -p ray-task`, compare with `main`:
1. Complete a task: it shows done ~0.6 s, then slides out; "Tarefa concluída · Desfazer" toast.
2. Undo while it lingers: it stays, not done.
3. Delete a task: it slides out, then "Tarefa apagada · Desfazer"; undo restores it selected.
4. Complete then delete the same task quickly: it goes straight to the delete slide-out.
5. A task with a time one minute ahead pulses once when the minute turns.

- [ ] **Step 11: Commit**

```bash
git add Cargo.lock crates/app-slint/Cargo.toml crates/app-slint/src/controller.rs crates/app-slint/src/bind.rs crates/app-slint/tests/controller.rs CONTRIBUTING.md
git commit -m "refactor(app): drive row animations with gus-anim-state"
```
