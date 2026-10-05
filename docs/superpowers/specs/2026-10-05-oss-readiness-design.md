# OSS readiness — design

Date: 2026-10-05
Status: draft, awaiting review

## Goal

Make ray-task ready for public open-source use: easy to install on any desktop OS,
easy to contribute to, and a README that makes people want to star and share it.

Success criteria:

- A user on Linux, macOS or Windows installs ray-task with one command or one MSI.
- A contributor goes from clone to green local checks with the steps in `CONTRIBUTING.md`.
- Every PR is gated by fmt, clippy, tests on 3 OSes, MSRV, coverage floor, cargo-deny
  and a Conventional Commit title.
- Releases happen by merging a release-plz PR; no manual tagging or packaging.
- The README hero (above the fold) explains and sells the app on its own.

## Decisions (from brainstorming)

| Topic | Choice |
|---|---|
| Distribution | `dist` (cargo-dist): GitHub Releases + shell/PowerShell installers + MSI |
| Git hooks | lefthook |
| CI guards | coverage floor, MSRV, Conventional Commits, cargo-deny (plus fmt/clippy/test) |
| Release trigger | release-plz → tag → dist |
| README language | English only; existing pt-BR README is replaced |
| README positioning | anti-Electron / low-RAM; author story = weak PC freezing under Electron apps |

Out of scope: code signing (macOS notarization, Windows Authenticode), Flatpak/Homebrew/AUR/
winget, UI i18n (app UI stays pt-BR), crates.io publishing, Codecov or other external
coverage services.

## Part 1 — development loop

### Hooks (`lefthook.yml`)

Installed with `lefthook install` (documented in CONTRIBUTING).

- `pre-commit`: `cargo fmt --all -- --check`, only when staged files match `*.rs`. No compilation.
- `commit-msg`: inline shell regex enforcing Conventional Commits:
  `^(feat|fix|chore|docs|refactor|test|perf|ci|build|style|revert)(\([a-z0-9-]+\))?!?: .+`
  Merge and `fixup!`/`squash!` messages are allowed through.
- `pre-push` (sequential, honors `CARGO_BUILD_JOBS`): `cargo clippy --workspace --all-targets -- -D warnings`,
  then `cargo test --workspace`.

Clippy is kept out of pre-commit on purpose: it compiles, which is slow on weak machines.

### CI (`.github/workflows/ci.yml`)

Triggers: push to `main`, pull requests. `concurrency` group per ref with
`cancel-in-progress: true`. `Swatinem/rust-cache` on every compiling job. Ubuntu jobs install
Slint system deps (`libfontconfig1-dev libxkbcommon-dev libwayland-dev`).

| Job | Runs on | Command |
|---|---|---|
| `fmt` | ubuntu | `cargo fmt --all -- --check` |
| `clippy` | ubuntu | `cargo clippy --workspace --all-targets -- -D warnings` |
| `test` | ubuntu, macos, windows | `cargo test --workspace` |
| `msrv` | ubuntu | toolchain `1.89`, `cargo check --workspace --all-targets` |
| `coverage` | ubuntu | `cargo llvm-cov --workspace --lcov --output-path lcov.info --fail-under-lines N`; upload `lcov.info` as artifact |
| `deny` | ubuntu | `EmbarkStudios/cargo-deny-action` with `deny.toml` |

PR title lint lives in `.github/workflows/pr-title.yml` using
`amannn/action-semantic-pull-request` (squash merges make the PR title the commit subject,
which release-plz parses).

### Guards

- **Coverage floor `N`:** measured once locally (`CARGO_BUILD_JOBS=2 cargo llvm-cov --workspace --summary-only`),
  floored to the integer below. Ratchet policy in CONTRIBUTING: raise `N` when coverage
  rises; never lower it without a written reason in the PR.
  The ignored perf test is not run under coverage.
- **`deny.toml`:** licenses allowed: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception,
  BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, OFL-1.1, MPL-2.0, CC0-1.0,
  plus any others found in the current tree and judged MIT-compatible (listed explicitly,
  each with a comment). Slint's license (GPL-3.0 / royalty-free / commercial) must be
  checked during implementation; if it needs a clarification entry, add it with a comment
  explaining the royalty-free desktop license applies. Advisories: deny. Sources: crates.io only.
  Duplicate versions: warn.
- **MSRV:** `rust-version = "1.89"` stays the contract; the `msrv` job enforces it.
- **Dependabot** (`.github/dependabot.yml`): `cargo` and `github-actions`, weekly, grouped
  into one PR per ecosystem, commit prefix `chore(deps)`.

## Part 2 — release and distribution

### Flow

```
merge PR → release-plz opens "chore: release vX.Y.Z" PR (version bump + CHANGELOG.md)
        → merge → release-plz pushes tag vX.Y.Z
        → dist release.yml builds all targets → GitHub Release with archives + installers
```

### release-plz

- `release-plz.toml`: workspace `publish = false`, `git_release_enable = false` (dist owns
  the GitHub Release), single changelog `CHANGELOG.md` at repo root.
- Only package `ray-task` gets a tag, named `v{{ version }}`; `ray-core` has tagging and its own
  changelog disabled, its changes are included in the root changelog. Versions stay shared via
  `workspace.package.version`.
- Both crates get `publish = false` in their `Cargo.toml`.
- `.github/workflows/release-plz.yml` runs on push to `main` with secret `RELEASE_PLZ_TOKEN`
  (fine-grained PAT, contents + pull-requests write). A tag pushed with `GITHUB_TOKEN` would not
  trigger the dist workflow, hence the PAT.

### dist

- Generated by `dist init`; config in `dist-workspace.toml`, workflow `.github/workflows/release.yml`.
- Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
  If `aarch64-unknown-linux-gnu` fails because of Slint system deps on the runner, drop it for the
  first release and note it in the plan's follow-ups.
- Installers: `shell`, `powershell`, `msi`.
- Archives include `LICENSE`, `README.md`, `packaging/ray-task.desktop`,
  `packaging/install-desktop.sh` and the icon PNGs.
- Linux system deps for the build job are added through dist's `dependencies.apt` config.

### Code changes

- `crates/app-slint/src/main.rs`: `#![cfg_attr(windows, windows_subsystem = "windows")]`
  so Windows does not open a console window.
- `packaging/install-desktop.sh`: copies `ray-task.desktop` and the hicolor icons into
  `~/.local/share` (paths relative to the script's own directory, works from the extracted
  archive). The shell installer only installs the binary, so this is the documented way to get
  a menu entry on Linux.

## Part 3 — README and community files

### README.md (English, replaces current)

Conversion goal: understand in seconds → install → star/share. Pricing principles do not
apply (MIT, free); "free, no account, no subscription" is used as an argument instead.

Sections, in order:

1. **Hero** (above the fold, must sell alone)
   - Logo, then headline: **"Your TODO app shouldn't need 500 MB of RAM."**
     The 500 MB figure must be backed by a real measurement of at least one popular Electron/web
     TODO app (see "Assets from the author"); if no measurement exists, use
     **"5,000 tasks. 40 MB. Zero accounts."** instead.
   - Subtitle: "ray-task is a keyboard-first TODO app for your desktop. Local SQLite, no account,
     no subscription. Written in Rust."
   - Demo GIF (~10 s, keyboard only): `Ctrl+N` → type → `Ctrl+D` `A` → `Ctrl+T` tag →
     `Ctrl+Enter` → `Ctrl+2`.
   - One CTA: "Install in one line →" anchor to Install, followed by the shell one-liner.
   - Badges on one line: CI, latest release, license, MSRV.
2. **The problem** — 3 lines: login walls, cloud sync nobody asked for, browser-sized RAM,
   a monthly fee for a text list.
3. **Numbers** — three figures, "with 5,000 tasks": `< 40 MB RAM`, `< 2 ms view switch`,
   `< 150 ms cold load`, linking to `crates/core/tests/perf.rs` and `docs/manual-checklist.md`.
4. **Comparison** — ray-task vs Todoist, TickTick, Things, Microsoft To Do. Columns: account
   required, subscription, works offline, open source, Linux, RAM (only measured values; the
   column is dropped if nothing was measured). Only verifiable facts.
5. **Keyboard** — shortcut table (translated from the current README).
6. **Install** — per-OS: shell one-liner (Linux/macOS), PowerShell one-liner and MSI (Windows),
   manual download, `cargo install --git`. Linux menu entry via `install-desktop.sh`.
   Unsigned-binary note: macOS `xattr -d com.apple.quarantine`, Windows SmartScreen "Run anyway".
   Note that the UI is currently in Brazilian Portuguese.
7. **Where your data lives** — table per OS (Linux / macOS / Windows), following `dirs`.
8. **Build from source** — system deps per distro (Fedora, Debian/Ubuntu, Arch), `cargo run`,
   GPU backend switch.
9. **Why this exists** — first person, by the author: weak PC that froze under Electron apps,
   wanted a TODO that does not steal RAM. Written in the author's voice; the author reviews it.
10. **Contributing** → CONTRIBUTING.md. **License** (MIT + Inter font OFL note).
11. **Shareable footer** — "If ray-task saved you a gigabyte of RAM, give it a ⭐" plus a
    pre-filled share link (X/Twitter intent URL with headline + repo URL).

### Social preview

1280×640 PNG at `docs/assets/social-preview.png`: app screenshot + headline. Built from an HTML
template rendered with Playwright. Uploading it in GitHub Settings → Social preview is a manual
step in `docs/maintainers.md`.

### Assets from the author

- Demo GIF at `docs/assets/demo.gif`. The plan provides the exact script and recording commands
  (`wf-recorder` → `gifski` on Wayland).
- Screenshot at `docs/assets/screenshot.png` (used in the social preview).
- Optional: RAM of competitor apps installed on the author's machine, measured the same way as
  ray-task (`/usr/bin/time -v` / `smem` → RSS).

The README references these paths; until they exist, the hero shows the screenshot only.

### Community files

- `CONTRIBUTING.md`: setup (rustup, system deps, `lefthook install`), commands (test, clippy,
  local coverage, perf budget), Conventional Commits, coverage ratchet, release flow,
  architecture in 3 lines (`ray-core` UI-agnostic, `app-slint` Slint shell).
- `.github/pull_request_template.md`: short checklist.
- `.github/ISSUE_TEMPLATE/bug.yml`, `feature.yml`, `config.yml`.
- `SECURITY.md`: report through GitHub private vulnerability reporting.
- `CODE_OF_CONDUCT.md`: Contributor Covenant 2.1.
- `docs/maintainers.md`: manual GitHub steps — branch protection on `main` requiring the CI
  checks, squash-merge only, `RELEASE_PLZ_TOKEN` secret, enable private vulnerability reporting,
  upload social preview.

## Order of work

1. Hooks (lefthook) + Windows subsystem attribute.
2. CI + guards (coverage baseline, deny.toml, dependabot, PR title).
3. release-plz + dist + install-desktop.sh.
4. Community files.
5. README + social preview (last, since it documents everything above).

## Risks

- Slint system deps on macOS/Windows runners and aarch64 Linux cross builds may need extra
  config; the CI `test` matrix surfaces this before the first release.
- cargo-deny may flag Slint's license; resolved via explicit clarification, never by
  disabling the license check.
- Local verification on the author's weak PC: one build at a time, `CARGO_BUILD_JOBS=2`.
