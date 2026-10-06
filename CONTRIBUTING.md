# Contributing to ray-task

Thanks for helping! This guide gets you from clone to a green PR.

## Setup

1. Install Rust with [rustup](https://rustup.rs) (stable; MSRV is 1.92).
2. System libraries for Slint (Linux only):
   - Fedora: `sudo dnf install fontconfig-devel libxkbcommon-devel wayland-devel`
   - Debian/Ubuntu: `sudo apt install libfontconfig1-dev libxkbcommon-dev libwayland-dev`
   - Arch: `sudo pacman -S fontconfig libxkbcommon wayland`
3. Install [lefthook](https://lefthook.dev/installation/) and enable the hooks:
   ```bash
   lefthook install
   ```
   - `pre-commit`: `cargo fmt --check` (only when `.rs` files are staged)
   - `commit-msg`: Conventional Commits check (`scripts/check-commit-msg.sh`)
   - `pre-push`: `cargo clippy -D warnings`, then `cargo test`

On a slow machine, cap parallel compilation: `export CARGO_BUILD_JOBS=2`.

## Everyday commands

| What | Command |
|---|---|
| Run the app | `cargo run -p ray-task --release` |
| All tests | `cargo test --workspace` |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format | `cargo fmt --all` |
| Coverage | `cargo llvm-cov --workspace --summary-only` |
| Performance budget | `cargo test -p ray-core --release --test perf -- --ignored --nocapture` |
| Dependency policy | `cargo deny check` |

## Architecture at a glance

- `crates/core` (`ray-core`): domain, store, undo and SQLite persistence. UI-agnostic, no Slint here.
- `crates/gus-list` (+ `gus-list-slint`): headless list logic (grouping, selection, keyed diff), std only; the first GusStack crate.
- `crates/gus-anim-state`: headless animation timing (`Timeline`: per-key scripts of timed phases), std only.
- `crates/gus-keys` (+ `gus-keys-slint`): headless keyboard shortcuts (chords + keymap), std only. ray-task's global shortcuts live in `crates/app-slint/src/keys.rs`, and a test keeps the README table in sync.
- `crates/app-slint` (`ray-task`): Slint UI, bindings and the app binary.
- New logic goes into `ray-core` with tests; the UI layer stays thin.

## Commits and PRs

- [Conventional Commits](https://www.conventionalcommits.org): `type(scope): summary`, e.g. `fix(app): close tag popover on Esc`. Types: feat, fix, chore, docs, refactor, test, perf, ci, build, style, revert.
- PRs are squash-merged, so **the PR title becomes the commit** and must follow the same format (CI checks it).
- `feat` bumps the minor version, `fix` the patch; `!` marks a breaking change.

## Coverage

CI fails if line coverage drops below **93%**. The floor only goes up: if your PR raises coverage, bump `--fail-under-lines` in `.github/workflows/ci.yml` to the new integer. Lowering it needs a written reason in the PR description.

## Releases (maintainers)

Merging to `main` makes release-plz open or update a "chore: release" PR with the version bump and `CHANGELOG.md`. Merging that PR tags `vX.Y.Z`; dist then builds binaries and installers and publishes the GitHub Release. See [`docs/maintainers.md`](docs/maintainers.md).
