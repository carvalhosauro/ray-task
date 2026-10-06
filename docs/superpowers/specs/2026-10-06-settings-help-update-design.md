# Settings, shortcut help and update notice — design

Date: 2026-10-06
Status: draft, awaiting review

## Goal

Three additions to ray-task that share one piece of navigation ("pages outside the task list"):

1. **Settings page** with the color scheme: System / Light / Dark. Today `Theme.dark` only follows
   `Palette.color-scheme`, so the user cannot override the OS.
2. **Shortcut help**: a quick overlay (`F1` / `?`) plus a section in Settings listing every
   shortcut, generated from the same source as the keymap.
3. **Update notice**: a quiet hint in the sidebar when a newer release exists on GitHub, with a
   popover that makes updating one click away (copy the installer command, or open the release).

Success: the user can force light/dark and it persists across restarts; every global shortcut is
discoverable inside the app in Portuguese; a new release shows up within a day without any
intrusive UI, and updating takes one paste in a terminal.

Out of scope: self-update (see *Future*), i18n beyond Portuguese, custom shortcuts, other
preferences (font, density).

## Decisions taken during brainstorming

- Navigation: **one Settings page with sections** (Aparência, Atualizações, Atalhos) opened by a
  gear in the sidebar footer or `Ctrl+,`, **plus** a shortcut overlay on `F1` / `?`.
- Update flow: **level 1** — notify + popover with "copy command" / "open release". Level 2
  (in-app "update and restart") is recorded under *Future*.
- Automatic update check: **on by default**, with a visible toggle and a README privacy note.
- Persistence: **SQLite `settings` table** in `ray-core`, written through the existing writer
  thread. Update logic is a **module in the app** (`src/update.rs`), not a GusStack crate yet
  (one consumer; extract when httpsnap/kestrel need it).

## 1. Data and core (`ray-core`)

Migration `002_settings.sql`:

```sql
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
```

Types:

```rust
pub enum ThemeMode { System, Light, Dark }          // default System

pub struct Settings {
    pub theme: ThemeMode,
    pub update_check: bool,                          // default true
    pub update_last_check: Option<NaiveDateTime>,    // last *successful* check
    pub update_dismissed: Option<String>,            // version the user said "don't remind me"
}
```

- `db::load` reads the table into `Snapshot.settings`. Unknown keys are ignored; an invalid value
  falls back to the default with `tracing::warn` (also covers a downgrade meeting newer keys).
- Key names and encodings: `theme` = `system|light|dark`, `update_check` = `0|1`,
  `update_last_check` = `%Y-%m-%d %H:%M:%S`, `update_dismissed` = semver string without `v`.
- `Store` gains `settings()`, `set_theme(ThemeMode)`, `set_update_check(bool)`,
  `mark_update_checked(NaiveDateTime)`, `dismiss_update(String)`. Each updates in-memory state and
  pushes `WriteOp::SetSetting { key, value }`, applied as
  `INSERT … ON CONFLICT(key) DO UPDATE SET value = excluded.value`.
- Settings are **not** undoable: no undo entry, and Ctrl+Z after a theme change undoes the last
  task operation as before.
- Write failures surface through the existing "Falha ao salvar" toast.

## 2. Update check (`crates/app-slint/src/update.rs`)

### Pure part (no network, fully unit-tested)

- `parse_release(json: &str) -> Result<Release, UpdateError>` with
  `Release { version: semver::Version, url: String, notes: String }`, from `tag_name` (leading
  `v` stripped), `html_url`, `body`. **Rejects** any `html_url` not starting with
  `https://github.com/carvalhosauro/ray-task/releases/` — the app never opens an arbitrary URL
  from the network.
- `is_newer(current: &Version, release: &Version, dismissed: Option<&str>) -> bool` — strictly
  greater than `CARGO_PKG_VERSION` and not equal to the dismissed version.
- `should_check(settings: &Settings, now: NaiveDateTime) -> bool` — toggle on and (never checked
  or ≥ 24 h since `update_last_check`).
- `notes_excerpt(body: &str) -> String` — first 5 non-empty lines, leading `#`, `-`, `*` and
  surrounding whitespace removed, each line cut at ~80 chars with `…`.
- `install_command(os: Os) -> &'static str`:
  - Linux / macOS:
    `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.sh | sh`
  - Windows:
    `powershell -ExecutionPolicy Bypass -c "irm https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.ps1 | iex"`

### Network shell

- `check() -> Result<Release, UpdateError>`: `ureq` GET
  `https://api.github.com/repos/carvalhosauro/ray-task/releases/latest`, 10 s timeout,
  `User-Agent: ray-task/<version>`, `Accept: application/vnd.github+json`.
- Runs on a `std::thread`; the result comes back with `slint::invoke_from_event_loop` into
  `bind`, which updates the UI and, on success, calls `mark_update_checked(now)`.

### When it runs

- 5 s after startup (keeps startup time untouched), if `should_check`.
- A repeating `Timer` every 6 h re-evaluates `should_check` (the app can stay open for days).
- "Verificar agora" in Settings ignores the 24 h window (but still requires no check in flight).

### Errors

- Network error, timeout, non-200, bad JSON, rejected URL: `tracing::warn`, nothing on screen for
  automatic checks. `update_last_check` is only written on success, so offline retries next cycle.
- Manual check shows a quiet status line: "Não foi possível verificar".

### New dependencies (app crate only)

`ureq` (rustls), `serde_json`, `semver`, `open` (open the browser). `ring` makes the first build
slower once; no runtime cost.

## 3. UI

### Navigation

- `AppWindow` gets `page: int` (0 = tasks, 1 = settings).
- Open Settings: gear icon in the sidebar footer (next to "Novo projeto") or `Ctrl+,`.
- Leave: `Esc`, or clicking any view/project in the sidebar. While on Settings no sidebar item is
  selected.
- Page switch reuses the 60 ms content crossfade from `switch_view`.

### Settings page

Same large header as the views ("Configurações"), then three sections:

- **Aparência** — segmented control `Sistema | Claro | Escuro`. `Theme` gets
  `in property <int> mode` (0 system, 1 light, 2 dark) and
  `dark: mode == 2 || (mode == 0 && Palette.color-scheme == ColorScheme.dark)`. Changing it applies
  immediately (existing color animations) and calls `set_theme`. The mode is applied on startup
  before the window shows, so there is no flash of the wrong theme.
- **Atualizações** — "Versão 0.1.0"; toggle "Verificar automaticamente" with a `Theme.sub` line
  "Consulta o GitHub uma vez por dia. Nada além disso é enviado."; button "Verificar agora" and a
  status line ("Você está na versão mais recente" / "Não foi possível verificar" / "Verificando…").
- **Atalhos** — the shortcut table component (shared with the overlay).

### Update notice

- Sidebar footer, above "Novo projeto": 6 px `Theme.accent` dot + "v0.2.0 disponível" in
  `Theme.sub`, 12 px. Fades in; no other motion.
- Click opens a popover (`MenuPanel`):
  - title "ray-task 0.2.0" and the notes excerpt;
  - **Copiar comando** (primary) — copies `install_command` for the current OS through a hidden
    `TextInput` (`select-all()` + `copy()`; Slint 1.18 has no clipboard API), label turns into
    "Copiado ✓" for 2 s;
  - **Ver release** — `open::that(url)` (URL already validated by `parse_release`);
  - small link "Não avisar desta versão" — `dismiss_update(version)`, hides the notice until a
    greater version appears.

### Shortcut overlay

- `F1` or `?` opens it — only from the root `FocusScope`, so typing `?` in a text field never
  triggers it.
- Centered card over a dimmed backdrop; `Esc` or click outside closes.
- Rows come from `keys::HELP`. `HelpRow` gains a `pt: &'static str` field: the README keeps using
  `text` (English), the app shows `pt`. One table, two languages.
- New bindings: `Ctrl+,` → `OpenSettings`, `F1` and `?` → `ShowHelp`. The existing
  `BINDINGS` ↔ `HELP` coverage test keeps the table complete.
- `gus-keys` needs function keys: add `Key::F(u8)` (parse `F1`…`F12`) with tests. Check during
  planning how `?` arrives from Slint (shifted char) so the chord matches with and without Shift.

## 4. Testing

- **Core**: migration over an existing v1 database; round-trip of every key; invalid value →
  default; setting changes do not touch the undo stack.
- **`update.rs`**: real GitHub JSON fixtures; rejected URL; version comparison incl. dismissed;
  24 h window; notes excerpt. No test touches the network.
- **UI** (`i-slint-backend-testing`, like the existing `ui_*` tests):
  - `Ctrl+,` opens Settings, `Esc` returns;
  - segmented control changes `Theme.mode` and persists through the store;
  - `F1` opens the overlay, `Esc` closes; `?` inside a text field does not open it;
  - an injected release (no network) shows the notice; "Não avisar desta versão" hides it and
    stores the version.
- **Manual** (`docs/manual-checklist.md`): light/dark/system, notice against a real release, paste
  the copied command in a terminal.

## 5. Docs

- README: shortcut table gains `Ctrl+,` and `F1`; a "Privacy" note on the daily check and how to
  turn it off.

## Implementation order

Each step ships on its own:

1. Core settings (migration, types, store, writer op).
2. Theme mode + Settings page + navigation.
3. Shortcut overlay (+ `gus-keys` function keys, `HelpRow.pt`).
4. Update: pure logic → network shell → sidebar notice + popover → Settings section.

## Future: level 2 — "Atualizar e reiniciar"

Set `install-updater = true` in `dist-workspace.toml`; installers then ship `ray-task-update`
(axoupdater) next to the binary. The popover gains a button that runs it, then relaunches the app.
Open questions before doing it:

- Do `.msi` installs write the install receipt axoupdater needs? If not, msi users stay on level 1.
- Windows cannot overwrite a running `.exe`: the app must exit first (spawn the updater detached,
  quit, updater relaunches).
- Users who installed before the change have no updater binary — level 1 remains the fallback.
