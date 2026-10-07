# Settings, Shortcut Help and Update Notice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Settings page (System/Light/Dark theme), an in-app shortcut help (overlay on `F1`/`?` + a section in Settings) and a quiet "new version available" notice with a popover that copies the installer command.

**Architecture:** Preferences live in a new SQLite `settings` table in `ray-core`, written through the existing writer thread. The app's `Controller` owns page/overlay/update-check state so it is unit-testable; a pure `update.rs` module parses GitHub's release JSON and decides when to check; a thin thread does the HTTP GET and hands the raw body back to the event loop through a Slint callback. Slint gets a new `Prefs` global, a `Theme.mode` input, `settings.slint` and `help.slint`.

**Tech Stack:** Rust 1.92, Slint 1.18, rusqlite, chrono, new deps `semver`, `serde_json`, `thiserror` (workspace), `ureq` 3, `open` 5.

**Spec:** `docs/superpowers/specs/2026-10-06-settings-help-update-design.md`

## Global Constraints

- The dev machine freezes under load: run **one cargo command at a time** and always prefix with `CARGO_BUILD_JOBS=2`.
- UI copy is Brazilian Portuguese; README/specs/commit messages are English; code comments follow the surrounding file (Portuguese in this repo).
- Settings are **not undoable**: never push an `UndoEntry` for them.
- The app only ever opens URLs starting with `https://github.com/carvalhosauro/ray-task/releases/`.
- Update check: on by default, at most once per 24 h automatically, 5 s after startup, re-evaluated every 6 h; failures of automatic checks are silent (log only).
- Install commands (exact):
  - Linux/macOS: `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.sh | sh`
  - Windows: `powershell -ExecutionPolicy Bypass -c "irm https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.ps1 | iex"`
- `keys.rs` stays the single source of shortcuts: the existing tests in `crates/app-slint/tests/keys.rs` (`help_rows_cover_exactly_the_keymap`, `help_labels_show_exactly_their_chords`, `readme_table_is_exactly_the_help_rows`) must keep passing.
- Every commit ends with:
  ```
  Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
  ```
- Before each commit: `cargo fmt` (lefthook checks it) and `CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets -- -D warnings`.

## Review Focus

1. **Keys while Settings is open must not touch the hidden task list** — `Delete` on the Settings page must not delete the selected task (tests: Task 4 controller test, Task 5 UI test).
2. **Typing `?` in a text field types a `?`** and does not open the overlay (test: Task 6 UI test with the filter field).
3. **System clock moved backwards** (`update_last_check` in the future) must not block checks forever (test: Task 7 `should_check_when_last_check_is_in_the_future`).
4. **Version edge cases**: same version, older release, dismissed version, then a newer one after dismissing (tests: Task 7 `is_newer_*`, Task 8 dismiss test).
5. **Duplicate or stale responses**: a manual click while a check is in flight does not start a second one; a response that arrives when no check is running is ignored (test: Task 8).

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/gus-keys/src/chord.rs` | add `Key::F(u8)` (parse/print `F1`…`F12`) |
| `crates/gus-keys-slint/src/lib.rs` | map Slint `F1`…`F12` to `Key::F` |
| `crates/core/src/migrations/002_settings.sql` | `settings` table |
| `crates/core/src/model.rs` | `ThemeMode`, `Settings`, setting key names, `Settings::apply` |
| `crates/core/src/ops.rs` | `WriteOp::SetSetting` |
| `crates/core/src/db.rs` | load/apply settings, register migration |
| `crates/core/src/store.rs` | `settings()`, `now_utc()`, setters |
| `crates/app-slint/src/controller.rs` | `Page`, `help_open`, theme/update-check setters, key gating, update-check state |
| `crates/app-slint/src/keys.rs` | `OpenSettings`, `ToggleHelp`, `HelpRow.pt`, `help_items()` |
| `crates/app-slint/src/update.rs` (new) | pure release parsing/decisions + `fetch()` |
| `crates/app-slint/src/bind.rs` | wire `Prefs`, `Theme.mode`, new callbacks, crossfade, update scheduling |
| `crates/app-slint/src/main.rs` | start update checks |
| `crates/app-slint/ui/theme.slint` | `Theme.mode` |
| `crates/app-slint/ui/types.slint` | `HelpItem` |
| `crates/app-slint/ui/globals.slint` | `Prefs` global + new `Actions` callbacks |
| `crates/app-slint/ui/components.slint` | `Segmented`, `Switch` |
| `crates/app-slint/ui/settings.slint` (new) | Settings page |
| `crates/app-slint/ui/help.slint` (new) | `ShortcutTable`, `HelpOverlay` |
| `crates/app-slint/ui/sidebar.slint` | settings button, update notice + popover, no highlight on Settings |
| `crates/app-slint/ui/app.slint` | `page`, `help-open`, mount page/overlay, exports |
| `crates/app-slint/ui/icons/sliders.svg` (new) | settings icon |
| `crates/app-slint/tests/fixtures/github-release.json` (new) | release API fixture |
| `crates/app-slint/tests/ui_settings.rs`, `ui_help.rs`, `ui_update.rs` (new) | UI tests |
| `README.md`, `docs/manual-checklist.md` | shortcut table, privacy note, manual checks |

---

### Task 1: Function keys in gus-keys

**Files:**
- Modify: `crates/gus-keys/src/chord.rs`
- Modify: `crates/gus-keys-slint/src/lib.rs`

**Interfaces:**
- Produces: `gus_keys::Key::F(u8)` (1–12); `"F1".parse::<Chord>()` works; `Chord` displays as `F1`; `gus_keys_slint::chord_from_slint` returns `Key::F(n)` for Slint `F1`…`F12`.

- [ ] **Step 1: Write the failing tests**

In `crates/gus-keys/src/chord.rs`, inside `mod tests`, add:

```rust
    #[test]
    fn function_keys_parse_and_print() {
        assert_eq!(parse("F1"), Chord::new(Key::F(1), Mods::default()));
        assert_eq!(parse("ctrl+f12"), Chord::new(Key::F(12), ctrl()));
        assert_eq!(parse("F1").to_string(), "F1");
        assert_eq!(parse("Ctrl+F12").to_string(), "Ctrl+F12");
        // "F" sozinho continua sendo a letra
        assert_eq!(parse("Ctrl+F"), Chord::new(Key::Char('f'), ctrl()));
    }

    #[test]
    fn function_keys_out_of_range_are_unknown() {
        for s in ["F0", "F13", "F99"] {
            assert_eq!(s.parse::<Chord>(), Err(ChordError::UnknownKey(s.to_string())), "{s}");
        }
    }
```

In `crates/gus-keys-slint/src/lib.rs`, replace the test `modifier_only_and_function_keys_are_none` with:

```rust
    #[test]
    fn modifier_only_and_other_special_keys_are_none() {
        for key in [SlintKey::Control, SlintKey::Shift, SlintKey::Alt, SlintKey::Meta, SlintKey::Insert] {
            assert_eq!(chord_from_slint(&text(key), true, false, false, false), None, "{key:?}");
        }
    }

    #[test]
    fn function_keys_map_to_f() {
        assert_eq!(chord_from_slint(&text(SlintKey::F1), false, false, false, false), plain(Key::F(1)));
        assert_eq!(chord_from_slint(&text(SlintKey::F12), false, false, false, false), plain(Key::F(12)));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_BUILD_JOBS=2 cargo test -p gus-keys -p gus-keys-slint`
Expected: compile error `no variant or associated item named F found for enum Key`.

- [ ] **Step 3: Implement**

`crates/gus-keys/src/chord.rs` — add the variant after `PageDown,`:

```rust
    /// Function keys `F1` to `F12`.
    F(u8),
```

In `parse_key`, before `let key = match name.to_lowercase().as_str() {`, add:

```rust
    let lower = name.to_lowercase();
    if let Some(n) = lower.strip_prefix('f').filter(|d| !d.is_empty()).and_then(|d| d.parse::<u8>().ok()) {
        return if (1..=12).contains(&n) { Ok(Key::F(n)) } else { Err(ChordError::UnknownKey(name.to_string())) };
    }
```

and change the `match` to use it: `let key = match lower.as_str() {`.

In `impl fmt::Display for Chord`, add before the `Key::Up => "Up",` arm:

```rust
            Key::F(n) => return write!(f, "F{n}"),
```

`crates/gus-keys-slint/src/lib.rs` — update the doc comment of `chord_from_slint` to say "(modifiers pressed alone, special keys other than F1–F12)" and, in `chord_from_slint`, replace

```rust
    let key = named(c).or_else(|| is_printable(c).then_some(Key::Char(c)))?;
```

with

```rust
    let key = named(c).or_else(|| function(c)).or_else(|| is_printable(c).then_some(Key::Char(c)))?;
```

and add below `fn named`:

```rust
fn function(c: char) -> Option<Key> {
    [
        SlintKey::F1,
        SlintKey::F2,
        SlintKey::F3,
        SlintKey::F4,
        SlintKey::F5,
        SlintKey::F6,
        SlintKey::F7,
        SlintKey::F8,
        SlintKey::F9,
        SlintKey::F10,
        SlintKey::F11,
        SlintKey::F12,
    ]
    .into_iter()
    .position(|k| char::from(k) == c)
    .map(|i| Key::F(i as u8 + 1))
}
```

If `crates/gus-keys/README.md` lists the supported key names, add `F1`–`F12` there.

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p gus-keys -p gus-keys-slint`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/gus-keys crates/gus-keys-slint
git commit -F - <<'EOF'
feat(gus-keys): support function keys F1 to F12

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 2: Settings in ray-core

**Files:**
- Create: `crates/core/src/migrations/002_settings.sql`
- Modify: `crates/core/src/model.rs`, `crates/core/src/ops.rs`, `crates/core/src/db.rs`, `crates/core/src/store.rs`
- Test: `crates/core/tests/db.rs`, `crates/core/tests/store_undo.rs`, unit tests in `model.rs`

**Interfaces:**
- Produces:
  - `ray_core::ThemeMode { System, Light, Dark }` (`Default` = `System`, `Copy`, `as_str()`, `parse(&str) -> Option<Self>`)
  - `ray_core::Settings { theme: ThemeMode, update_check: bool, update_last_check: Option<DateTime<Utc>>, update_dismissed: Option<String> }` (`Default`: `System, true, None, None`), `Settings::apply(&mut self, key: &str, value: &str) -> Result<(), String>`
  - `Snapshot.settings: Settings`
  - `WriteOp::SetSetting { key: &'static str, value: String }`
  - `Store::settings(&self) -> &Settings`, `Store::now_utc(&self) -> DateTime<Utc>`, `Store::set_theme(&mut self, ThemeMode)`, `Store::set_update_check(&mut self, bool)`, `Store::mark_update_checked(&mut self)`, `Store::dismiss_update(&mut self, version: String)`

- [ ] **Step 1: Write the failing tests**

Unit tests at the end of `mod tests` in `crates/core/src/model.rs`:

```rust
    #[test]
    fn settings_apply_known_keys() {
        let mut s = Settings::default();
        s.apply("theme", "dark").unwrap();
        s.apply("update_check", "0").unwrap();
        s.apply("update_last_check", "2026-10-06T10:00:00Z").unwrap();
        s.apply("update_dismissed", "0.2.0").unwrap();
        assert_eq!(s.theme, ThemeMode::Dark);
        assert!(!s.update_check);
        assert_eq!(s.update_last_check.unwrap().to_rfc3339(), "2026-10-06T10:00:00+00:00");
        assert_eq!(s.update_dismissed.as_deref(), Some("0.2.0"));
    }

    #[test]
    fn settings_reject_invalid_values_and_ignore_unknown_keys() {
        let mut s = Settings::default();
        assert!(s.apply("theme", "purple").is_err());
        assert!(s.apply("update_check", "maybe").is_err());
        assert!(s.apply("update_last_check", "ontem").is_err());
        assert!(s.apply("some_future_key", "x").is_ok());
        assert_eq!(s, Settings::default());
    }
```

Append to `crates/core/tests/db.rs` (add `use ray_core::{FixedClock, Settings, Snapshot, Store, ThemeMode};` to the imports):

```rust
#[test]
fn settings_roundtrip() {
    let mut conn = db::open_in_memory().unwrap();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-06 10:00")));
    store.set_theme(ThemeMode::Dark);
    store.set_update_check(false);
    store.mark_update_checked();
    store.dismiss_update("0.2.0".into());
    db::apply_all(&mut conn, &store.take_ops()).unwrap();
    let snap = db::load(&conn).unwrap();
    assert_eq!(
        snap.settings,
        Settings {
            theme: ThemeMode::Dark,
            update_check: false,
            update_last_check: Some(ts("2026-10-06 10:00")),
            update_dismissed: Some("0.2.0".into()),
        }
    );
}

#[test]
fn setting_written_twice_keeps_the_last_value() {
    let mut conn = db::open_in_memory().unwrap();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-06 10:00")));
    store.set_theme(ThemeMode::Dark);
    store.set_theme(ThemeMode::Light);
    db::apply_all(&mut conn, &store.take_ops()).unwrap();
    assert_eq!(db::load(&conn).unwrap().settings.theme, ThemeMode::Light);
}

#[test]
fn invalid_setting_values_fall_back_to_defaults() {
    let conn = db::open_in_memory().unwrap();
    conn.execute_batch(
        "INSERT INTO settings (key, value) VALUES ('theme', 'purple'), ('update_check', 'maybe'), ('future_key', 'x');",
    )
    .unwrap();
    assert_eq!(db::load(&conn).unwrap().settings, Settings::default());
}

#[test]
fn version_1_database_gains_settings_with_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ray-task.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(include_str!("../src/migrations/001_init.sql")).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, color, sort_order, created_at) VALUES (1, 'Casa', '#0A84FF', 0, '2026-10-01T09:00:00Z')",
            [],
        )
        .unwrap();
    }
    let conn = db::open(&path).unwrap();
    let snap = db::load(&conn).unwrap();
    assert_eq!(snap.projects.len(), 1);
    assert_eq!(snap.settings, Settings::default());
    assert!(path.with_extension("db.bak").exists(), "backup antes de migrar");
}
```

Append to `crates/core/tests/store_undo.rs` (adjust imports to include `ThemeMode`, `View`, `FixedClock`, `Snapshot`, `Store` as needed):

```rust
#[test]
fn settings_are_not_undoable() {
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-06 10:00")));
    let id = store.create_task(View::Inbox, "A").unwrap();
    store.delete_task(id).unwrap();
    store.set_theme(ThemeMode::Dark);
    assert!(store.undo(), "desfaz o apagar, não o tema");
    assert!(store.task(id).is_some());
    assert_eq!(store.settings().theme, ThemeMode::Dark);
    assert!(!store.undo());
}

#[test]
fn setting_the_same_value_writes_nothing() {
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-06 10:00")));
    store.take_ops();
    store.set_theme(ThemeMode::System);
    store.set_update_check(true);
    assert!(store.take_ops().is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-core`
Expected: compile errors (`Settings`, `ThemeMode` not found).

- [ ] **Step 3: Implement**

`crates/core/src/migrations/002_settings.sql`:

```sql
-- Preferências do app (chave/valor). Valores inválidos voltam ao padrão na leitura.
CREATE TABLE settings (
  key    TEXT PRIMARY KEY,
  value  TEXT NOT NULL
);
```

`crates/core/src/db.rs`:

```rust
const MIGRATIONS: &[&str] = &[include_str!("migrations/001_init.sql"), include_str!("migrations/002_settings.sql")];
```

At the end of `load`, before `Ok(snap)`:

```rust
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (key, value) = row?;
        if let Err(error) = snap.settings.apply(&key, &value) {
            tracing::warn!(%key, %value, %error, "configuração inválida, usando o padrão");
        }
    }
```

In `apply_in`, add the arm:

```rust
        WriteOp::SetSetting { key, value } => {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
```

`crates/core/src/ops.rs` — add to the enum:

```rust
    /// Preferência (tabela `settings`); a chave é uma das `SETTING_*` de `model`.
    SetSetting { key: &'static str, value: String },
```

`crates/core/src/model.rs` — add (and `Settings` to `Snapshot`):

```rust
pub const SETTING_THEME: &str = "theme";
pub const SETTING_UPDATE_CHECK: &str = "update_check";
pub const SETTING_UPDATE_LAST_CHECK: &str = "update_last_check";
pub const SETTING_UPDATE_DISMISSED: &str = "update_dismissed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark].into_iter().find(|m| m.as_str() == s)
    }
}

/// Preferências do app. Não entram no desfazer.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub theme: ThemeMode,
    pub update_check: bool,
    /// Última verificação de atualização que deu certo.
    pub update_last_check: Option<DateTime<Utc>>,
    /// Versão que o usuário pediu para não ser avisado.
    pub update_dismissed: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: ThemeMode::System, update_check: true, update_last_check: None, update_dismissed: None }
    }
}

impl Settings {
    /// Aplica uma linha da tabela `settings`. Chave desconhecida é ignorada (banco de uma versão
    /// mais nova); valor inválido é erro e não muda nada.
    pub fn apply(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            SETTING_THEME => self.theme = ThemeMode::parse(value).ok_or_else(|| format!("tema '{value}'"))?,
            SETTING_UPDATE_CHECK => {
                self.update_check = match value {
                    "1" => true,
                    "0" => false,
                    _ => return Err(format!("booleano '{value}'")),
                }
            }
            SETTING_UPDATE_LAST_CHECK => {
                let when = DateTime::parse_from_rfc3339(value).map_err(|e| format!("data/hora '{value}': {e}"))?;
                self.update_last_check = Some(when.with_timezone(&Utc));
            }
            SETTING_UPDATE_DISMISSED => self.update_dismissed = Some(value.to_string()),
            _ => {}
        }
        Ok(())
    }
}
```

and in `Snapshot` add the field `pub settings: Settings,` (it already derives `Default`).

`crates/core/src/store.rs`:
- add field `settings: Settings,` to `Store`, initialised in `new` with `settings: snapshot.settings,` (move it out before `snapshot.tags`/`tasks` are consumed: bind `let settings = snapshot.settings;` at the top of `new`).
- add `use chrono::SecondsFormat;` to the chrono import.
- add a section:

```rust
    // ---------- preferências (sem desfazer) ----------

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn now_utc(&self) -> DateTime<Utc> {
        self.clock.now_utc()
    }

    pub fn set_theme(&mut self, theme: ThemeMode) {
        if self.settings.theme != theme {
            self.settings.theme = theme;
            self.push_setting(SETTING_THEME, theme.as_str().to_string());
        }
    }

    pub fn set_update_check(&mut self, on: bool) {
        if self.settings.update_check != on {
            self.settings.update_check = on;
            self.push_setting(SETTING_UPDATE_CHECK, if on { "1" } else { "0" }.to_string());
        }
    }

    pub fn mark_update_checked(&mut self) {
        let now = self.clock.now_utc();
        self.settings.update_last_check = Some(now);
        self.push_setting(SETTING_UPDATE_LAST_CHECK, now.to_rfc3339_opts(SecondsFormat::Secs, true));
    }

    pub fn dismiss_update(&mut self, version: String) {
        self.settings.update_dismissed = Some(version.clone());
        self.push_setting(SETTING_UPDATE_DISMISSED, version);
    }

    fn push_setting(&mut self, key: &'static str, value: String) {
        self.ops.push(WriteOp::SetSetting { key, value });
    }
```

Run `grep -rn "WriteOp::" crates --include='*.rs'` and add a `SetSetting` arm to any other exhaustive `match` on `WriteOp` the compiler reports.

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-core`
Expected: all pass, including the existing `fresh_database_is_migrated_to_current_version` (now version 2).

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -F - <<'EOF'
feat(core): store app settings in a settings table

Theme mode and update-check state live in a key/value table written
through the existing writer. Unknown keys are ignored and invalid values
fall back to defaults. Settings never enter the undo stack.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 3: Controller navigation and preference state

**Files:**
- Modify: `crates/app-slint/src/controller.rs`
- Test: `crates/app-slint/tests/controller.rs`

**Interfaces:**
- Consumes: `Store::set_theme`, `Store::set_update_check`, `ThemeMode` (Task 2).
- Produces:
  - `pub enum Page { Tasks, Settings }` (`Debug, Clone, Copy, PartialEq, Eq`) with `pub fn as_int(self) -> i32` (Tasks 0, Settings 1)
  - `Controller` fields `pub page: Page` (init `Tasks`), `pub help_open: bool` (init `false`)
  - `Controller::open_settings(&mut self)`, `Controller::toggle_help(&mut self)`, `Controller::set_theme(&mut self, ThemeMode)`, `Controller::set_update_check(&mut self, bool)`
  - `Controller::select_view` now also sets `page = Page::Tasks`
  - `Controller::escape` closes the overlay first, then leaves Settings, then the old behaviour.

- [ ] **Step 1: Write the failing tests**

Append to `crates/app-slint/tests/controller.rs` (add `use ray_core::ThemeMode;` and `use ray_task::controller::Page;`):

```rust
#[test]
fn escape_closes_help_then_settings_then_task_state() {
    let mut f = at("2026-10-05 13:35");
    f.c.start_adding();
    f.c.open_settings();
    f.c.toggle_help();
    assert!(f.c.help_open);
    f.c.escape();
    assert!(!f.c.help_open);
    assert_eq!(f.c.page, Page::Settings);
    f.c.escape();
    assert_eq!(f.c.page, Page::Tasks);
    assert!(f.c.adding, "o Esc que saiu das configurações não mexe na lista");
    f.c.escape();
    assert!(!f.c.adding);
}

#[test]
fn selecting_a_view_leaves_settings() {
    let mut f = at("2026-10-05 13:35");
    f.c.open_settings();
    f.c.select_view(View::Inbox);
    assert_eq!(f.c.page, Page::Tasks);
}

#[test]
fn theme_and_update_toggle_are_persisted() {
    let mut f = at("2026-10-05 13:35");
    f.c.set_theme(ThemeMode::Dark);
    f.c.set_update_check(false);
    assert_eq!(f.c.store.settings().theme, ThemeMode::Dark);
    assert!(!f.c.store.settings().update_check);
    let sent = f.sent.borrow();
    assert!(sent.contains(&WriteOp::SetSetting { key: "theme", value: "dark".into() }));
    assert!(sent.contains(&WriteOp::SetSetting { key: "update_check", value: "0".into() }));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test controller`
Expected: compile error (`Page` / `open_settings` not found).

- [ ] **Step 3: Implement**

In `crates/app-slint/src/controller.rs`:

```rust
/// Página no lugar da lista de tarefas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Tasks,
    Settings,
}

impl Page {
    pub fn as_int(self) -> i32 {
        match self {
            Page::Tasks => 0,
            Page::Settings => 1,
        }
    }
}
```

Add fields to `Controller` (after `deleted`):

```rust
    pub page: Page,
    /// Overlay de atalhos (F1 / ?).
    pub help_open: bool,
```

initialised as `page: Page::Tasks, help_open: false,` in `new`. Import `ThemeMode` from `ray_core`.

Methods (next to `select_view`):

```rust
    pub fn open_settings(&mut self) {
        self.page = Page::Settings;
        self.popover_task = None;
    }

    pub fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
    }

    pub fn set_theme(&mut self, theme: ThemeMode) {
        self.store.set_theme(theme);
        self.flush();
    }

    pub fn set_update_check(&mut self, on: bool) {
        self.store.set_update_check(on);
        self.flush();
    }
```

In `select_view`, add `self.page = Page::Tasks;`.

At the top of `escape`, before `if self.adding {`:

```rust
        if self.help_open {
            self.help_open = false;
            return;
        }
        if self.page == Page::Settings {
            self.page = Page::Tasks;
            return;
        }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test controller`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/app-slint/src/controller.rs crates/app-slint/tests/controller.rs
git commit -F - <<'EOF'
feat(app): track settings page, help overlay and theme in the controller

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 4: Shortcuts for settings and help, Portuguese help text, key gating

**Files:**
- Modify: `crates/app-slint/src/keys.rs`, `crates/app-slint/src/controller.rs`, `crates/app-slint/src/bind.rs`, `crates/app-slint/ui/globals.slint`, `README.md`
- Test: `crates/app-slint/tests/keys.rs`, `crates/app-slint/tests/controller.rs`

**Interfaces:**
- Consumes: `Page`, `open_settings`, `toggle_help` (Task 3); `Key::F` (Task 1).
- Produces:
  - `KeyAction::OpenSettings`, `KeyAction::ToggleHelp`
  - `HelpRow { chords, label, text, pt }` — `pt: &'static str` is the Portuguese text shown in the app
  - `pub fn help_items() -> Vec<(String, String)>` — `(keys, text)` for the UI, backticks removed
  - `Controller::key_allowed(&self, action: KeyAction) -> bool`
  - Slint `Actions.open-settings()` and `Actions.toggle-help()` callbacks, wired in `bind.rs`.

- [ ] **Step 1: Write the failing tests**

In `crates/app-slint/tests/keys.rs`, in `keymap_has_every_global_shortcut` change `21` to `24` and add:

```rust
    assert_eq!(map.lookup(&chord("Ctrl+,")), Some(&KeyAction::OpenSettings));
    assert_eq!(map.lookup(&chord("F1")), Some(&KeyAction::ToggleHelp));
    assert_eq!(map.lookup(&chord("?")), Some(&KeyAction::ToggleHelp));
```

Add:

```rust
#[test]
fn question_mark_with_shift_still_opens_help() {
    // Em layouts US o `?` sai com Shift: o adaptador descarta o Shift de símbolos.
    let chord = gus_keys_slint::chord_from_slint("?", false, true, false, false).unwrap();
    assert_eq!(keymap().lookup(&chord), Some(&KeyAction::ToggleHelp));
}

#[test]
fn help_items_are_portuguese_without_backticks() {
    let items = ray_task::keys::help_items();
    assert_eq!(items.len(), HELP.len());
    assert_eq!(items[0], ("Ctrl+N".to_string(), "Nova tarefa na visão atual".to_string()));
    assert!(items.iter().all(|(k, t)| !k.contains('`') && !t.contains('`')));
    assert!(items.iter().any(|(k, _)| k == "F1 / ?"));
}
```

Append to `crates/app-slint/tests/controller.rs` (add `use ray_task::keys::KeyAction;`):

```rust
#[test]
fn task_keys_do_nothing_while_settings_is_open() {
    let mut f = at("2026-10-05 13:35");
    f.c.open_settings();
    for action in [
        KeyAction::DeleteSelected,
        KeyAction::ToggleSelected,
        KeyAction::MoveSelection(1),
        KeyAction::ExpandSelected,
        KeyAction::DateSelected,
        KeyAction::TagSelected,
        KeyAction::Undo,
        KeyAction::NewTask,
        KeyAction::ToggleFilter,
    ] {
        assert!(!f.c.key_allowed(action), "{action:?}");
    }
    for action in [KeyAction::Escape, KeyAction::ToggleHelp, KeyAction::OpenSettings, KeyAction::SelectNav(0), KeyAction::NewProject] {
        assert!(f.c.key_allowed(action), "{action:?}");
    }
}

#[test]
fn only_escape_and_help_work_while_help_is_open() {
    let mut f = at("2026-10-05 13:35");
    f.c.toggle_help();
    assert!(f.c.key_allowed(KeyAction::Escape));
    assert!(f.c.key_allowed(KeyAction::ToggleHelp));
    assert!(!f.c.key_allowed(KeyAction::SelectNav(0)));
    assert!(!f.c.key_allowed(KeyAction::OpenSettings));
    assert!(!f.c.key_allowed(KeyAction::NewTask));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test keys --test controller`
Expected: compile errors (`OpenSettings`, `key_allowed`, `help_items` not found).

- [ ] **Step 3: Implement**

`crates/app-slint/src/keys.rs`:
- Add `OpenSettings,` and `ToggleHelp,` to `KeyAction`.
- Add `pt` to `HelpRow`:

```rust
/// Uma linha da tabela de atalhos: os atalhos que ela documenta, o rótulo, o texto do README
/// (inglês) e o texto mostrado no app (português).
pub struct HelpRow {
    pub chords: &'static [&'static str],
    pub label: &'static str,
    pub text: &'static str,
    pub pt: &'static str,
}
```

- Append to `BINDINGS`:

```rust
    ("Ctrl+,", OpenSettings),
    ("F1", ToggleHelp),
    ("?", ToggleHelp),
```

- Replace `HELP` with (README `text` unchanged for existing rows):

```rust
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
```

`README.md` — in the "Keyboard first" table, after the `Ctrl+F` row add:

```markdown
| `Ctrl+,` | Settings |
| `F1` / `?` | Shortcut list |
```

`crates/app-slint/src/controller.rs` — import `crate::keys::KeyAction` and add:

```rust
    /// Atalho permitido agora? Com o overlay aberto, só Esc e F1/?; nas configurações, nada que
    /// mexa na lista escondida (a tarefa selecionada continua lá, fora de vista).
    pub fn key_allowed(&self, action: KeyAction) -> bool {
        if self.help_open {
            return matches!(action, KeyAction::Escape | KeyAction::ToggleHelp);
        }
        match self.page {
            Page::Tasks => true,
            Page::Settings => matches!(
                action,
                KeyAction::Escape | KeyAction::ToggleHelp | KeyAction::OpenSettings | KeyAction::SelectNav(_) | KeyAction::NewProject
            ),
        }
    }
```

`crates/app-slint/ui/globals.slint` — in `Actions`, under `// navegação`:

```slint
    callback open-settings();
    callback toggle-help();
```

`crates/app-slint/src/bind.rs`:
- In `run_key_action` add:

```rust
        KeyAction::OpenSettings => actions.invoke_open_settings(),
        KeyAction::ToggleHelp => actions.invoke_toggle_help(),
```

- In `wire_keyboard`, change the `on_key` closure to also check the controller (capture `s` instead of only `weak`):

```rust
    {
        let s = s.clone();
        let keymap = keys::keymap();
        actions.on_key(move |text, ctrl, shift, alt, meta| {
            let Some(ui) = s.ui.upgrade() else { return false };
            let Some(&action) = gus_keys_slint::chord_from_slint(&text, ctrl, shift, alt, meta).and_then(|chord| keymap.lookup(&chord))
            else {
                return false;
            };
            // Atalho conhecido mas bloqueado (configurações/overlay abertos): consome sem agir.
            if s.ctrl.borrow().key_allowed(action) {
                run_key_action(&ui.global::<Actions>(), action);
            }
            true
        });
    }
```

- Wire the two callbacks (in `wire_keyboard`; the page switch gets its crossfade in Task 5):

```rust
    {
        let s = s.clone();
        actions.on_open_settings(move || update(&s, |c| c.open_settings()));
    }
    {
        let s = s.clone();
        actions.on_toggle_help(move || update(&s, |c| c.toggle_help()));
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task`
Expected: all pass (including `readme_table_is_exactly_the_help_rows` and `help_labels_show_exactly_their_chords`).

- [ ] **Step 5: Commit**

```bash
git add crates/app-slint/src/keys.rs crates/app-slint/src/controller.rs crates/app-slint/src/bind.rs crates/app-slint/ui/globals.slint crates/app-slint/tests README.md
git commit -F - <<'EOF'
feat(app): add Ctrl+, and F1/? shortcuts with Portuguese help text

Keys that act on the selected task are ignored while the settings page
or the shortcut overlay is open, so nothing changes out of sight.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 5: Settings page, theme mode and sidebar button

**Files:**
- Create: `crates/app-slint/ui/settings.slint`, `crates/app-slint/ui/icons/sliders.svg`
- Modify: `crates/app-slint/ui/theme.slint`, `crates/app-slint/ui/types.slint`, `crates/app-slint/ui/globals.slint`, `crates/app-slint/ui/components.slint`, `crates/app-slint/ui/sidebar.slint`, `crates/app-slint/ui/app.slint`, `crates/app-slint/src/bind.rs`
- Test: `crates/app-slint/tests/ui_settings.rs`

**Interfaces:**
- Consumes: `Page`, `open_settings`, `set_theme`, `set_update_check`, `key_allowed` (Tasks 3–4).
- Produces:
  - Slint: `Theme.mode` (`in int`: 0 system, 1 light, 2 dark); `Prefs` global with `theme-mode`, `update-check`, `version`, `update-status`, `notice-version`, `notice-notes`, `install-command`, `help-rows: [HelpItem]`; `HelpItem { keys: string, text: string }`; `Actions.set-theme(int)`, `Actions.set-update-check(bool)`, `Actions.check-updates()`; `AppWindow.page: int`, `AppWindow.help-open: bool`; components `Segmented`, `Switch`.
  - Rust: `fn theme_int(ThemeMode) -> i32` / `fn theme_from_int(i32) -> ThemeMode` in `bind.rs`; `fn crossfade(s, change)` in `bind.rs`.

- [ ] **Step 1: Write the failing test**

`crates/app-slint/tests/ui_settings.rs`:

```rust
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use ray_core::{FixedClock, Settings, Snapshot, Store, ThemeMode, View, WriteOp};
use ray_task::bind;
use ray_task::controller::{Controller, Page};
use ray_task::{Actions, AppWindow, Theme};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};

fn ms(n: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(n));
}

fn tap(ui: &AppWindow, key: impl Into<SharedString>) {
    let key: SharedString = key.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: key });
    ms(20);
}

fn ctrl(ui: &AppWindow, key: impl Into<SharedString>) {
    let control: SharedString = Key::Control.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    tap(ui, key);
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: control });
}

fn click(ui: &AppWindow, label: &str) {
    let found: Vec<_> = ElementHandle::find_by_accessible_label(ui, label).collect();
    assert_eq!(found.len(), 1, "um elemento com o rótulo {label:?}");
    found[0].invoke_accessible_default_action();
}

#[test]
fn settings_page_switches_theme_and_persists_it() {
    i_slint_backend_testing::init_no_event_loop();
    let sent = Rc::new(RefCell::new(Vec::<WriteOp>::new()));
    let sink = sent.clone();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(move |ops| sink.borrow_mut().extend(ops))), Box::new(|| {}));
    ui.show().unwrap();
    ui.invoke_focus_root();

    ctrl(&ui, ",");
    ms(100);
    assert_eq!(ui.get_page(), 1);
    assert_eq!(ui.get_selected_nav(), -1, "nenhuma visão marcada nas configurações");

    click(&ui, "Escuro");
    assert_eq!(ui.global::<Theme>().get_mode(), 2);
    assert!(ui.global::<Theme>().get_dark());
    assert_eq!(binding.controller().store.settings().theme, ThemeMode::Dark);
    assert!(sent.borrow().contains(&WriteOp::SetSetting { key: "theme", value: "dark".into() }));

    click(&ui, "Claro");
    assert!(!ui.global::<Theme>().get_dark());

    tap(&ui, Key::Escape);
    assert_eq!(ui.get_page(), 0);
    assert_eq!(binding.controller().page, Page::Tasks);
}

#[test]
fn saved_theme_is_applied_before_the_window_shows() {
    i_slint_backend_testing::init_no_event_loop();
    let snapshot = Snapshot { settings: Settings { theme: ThemeMode::Dark, ..Settings::default() }, ..Snapshot::default() };
    let store = Store::new(snapshot, Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    assert_eq!(ui.global::<Theme>().get_mode(), 2);
    assert!(ui.global::<Theme>().get_dark());
}

#[test]
fn delete_on_the_settings_page_does_not_touch_the_hidden_task() {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    store.create_task(View::Today, "Comprar pão").unwrap();
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    ui.show().unwrap();
    ui.invoke_focus_root();
    ms(50);

    tap(&ui, Key::DownArrow);
    assert!(binding.controller().selected.is_some());
    ui.global::<Actions>().invoke_open_settings();
    ms(100);
    tap(&ui, Key::Delete);
    ms(300);
    assert_eq!(ui.get_tasks().row_count(), 1, "a tarefa escondida continua lá");
}

#[test]
fn update_check_switch_persists() {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    ui.show().unwrap();
    ui.global::<Actions>().invoke_open_settings();
    ms(100);
    click(&ui, "Verificar automaticamente");
    assert!(!binding.controller().store.settings().update_check);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test ui_settings`
Expected: compile error (`Theme` not exported / `get_page` not found).

- [ ] **Step 3: Implement the Slint side**

`crates/app-slint/ui/theme.slint` — add `mode` and change `dark`:

```slint
export global Theme {
    // 0 = segue o sistema, 1 = claro, 2 = escuro (Configurações › Aparência)
    in property <int> mode: 0;
    out property <bool> dark: mode == 2 || (mode == 0 && Palette.color-scheme == ColorScheme.dark);
```

`crates/app-slint/ui/types.slint` — add:

```slint
// Linha da ajuda de atalhos: teclas e o que fazem.
export struct HelpItem { keys: string, text: string }
```

`crates/app-slint/ui/globals.slint` — import `HelpItem` and add to `Actions`:

```slint
    // configurações e atualização
    callback set-theme(int);
    callback set-update-check(bool);
    callback check-updates();
```

and a new global:

```slint
// Estado das configurações e do aviso de atualização (preenchido pelo Rust).
export global Prefs {
    in property <int> theme-mode;
    in property <bool> update-check: true;
    in property <string> version;
    in property <string> update-status;
    // versão nova disponível ("" = nenhuma) e o resumo das notas
    in property <string> notice-version;
    in property <string> notice-notes;
    in property <string> install-command;
    in property <[HelpItem]> help-rows;
}
```

`crates/app-slint/ui/components.slint` — add:

```slint
/// Botões lado a lado, um selecionado (ex.: Sistema | Claro | Escuro).
export component Segmented inherits Rectangle {
    in property <[string]> options;
    in property <int> selected;
    callback picked(int);
    height: 30px;
    border-radius: 8px;
    background: Theme.field;
    HorizontalLayout {
        padding: 2px;
        spacing: 2px;
        for option[i] in root.options : Rectangle {
            min-width: 88px;
            border-radius: 6px;
            background: i == root.selected ? Theme.card : touch.has-hover ? Theme.selection.with-alpha(0.5) : transparent;
            drop-shadow-blur: i == root.selected ? 3px : 0px;
            drop-shadow-color: Theme.shadow;
            animate background { duration: 150ms; easing: ease-out; }
            accessible-role: button;
            accessible-label: option;
            accessible-action-default => { root.picked(i); }
            touch := TouchArea { clicked => { root.picked(i); } }
            Text {
                // o rótulo já está no botão: o leitor de tela (e os testes) o acham uma vez só
                accessible-role: none;
                accessible-label: "";
                text: option;
                font-size: 13px;
                font-weight: i == root.selected ? 600 : 400;
                color: Theme.text;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
    }
}

/// Interruptor liga/desliga.
export component Switch inherits Rectangle {
    in property <bool> checked;
    in property <string> label;
    callback toggled(bool);
    width: 36px;
    height: 22px;
    border-radius: 11px;
    background: root.checked ? Theme.accent : Theme.ring;
    animate background { duration: 150ms; easing: ease-out; }
    accessible-role: checkbox;
    accessible-checked: root.checked;
    accessible-label: root.label;
    accessible-action-default => { root.toggled(!root.checked); }
    TouchArea { clicked => { root.toggled(!root.checked); } }
    Rectangle {
        x: root.checked ? parent.width - self.width - 2px : 2px;
        y: 2px;
        width: 18px;
        height: 18px;
        border-radius: 9px;
        background: white;
        animate x { duration: 150ms; easing: ease-out; }
    }
}
```

`crates/app-slint/ui/settings.slint` (new):

```slint
import { Theme } from "theme.slint";
import { Actions, Prefs } from "globals.slint";
import { Segmented, Switch, DialogButton } from "components.slint";

component Section inherits VerticalLayout {
    in property <string> title;
    spacing: 10px;
    Text { text: root.title; font-size: 15px; font-weight: 600; color: Theme.text; }
    Rectangle { height: 1px; background: Theme.line; }
    @children
}

/// Configurações: ocupa o lugar da lista de tarefas (Esc ou a barra lateral voltam).
export component SettingsPage inherits Rectangle {
    VerticalLayout {
        HorizontalLayout {
            padding-left: 32px;
            padding-right: 24px;
            padding-top: 28px;
            padding-bottom: 10px;
            Text { text: "Configurações"; font-size: 26px; font-weight: 700; letter-spacing: -0.5px; color: Theme.text; }
        }
        Flickable {
            vertical-stretch: 1;
            viewport-height: body.preferred-height;
            body := VerticalLayout {
                alignment: start;
                padding-left: 32px;
                padding-right: 32px;
                padding-top: 8px;
                padding-bottom: 32px;
                spacing: 28px;
                Section {
                    title: "Aparência";
                    HorizontalLayout {
                        alignment: start;
                        Segmented {
                            options: ["Sistema", "Claro", "Escuro"];
                            selected: Prefs.theme-mode;
                            picked(i) => { Actions.set-theme(i); }
                        }
                    }
                }
                Section {
                    title: "Atualizações";
                    Text { text: "Versão " + Prefs.version; font-size: 13.5px; color: Theme.text; }
                    HorizontalLayout {
                        alignment: start;
                        spacing: 10px;
                        VerticalLayout {
                            alignment: center;
                            Switch {
                                label: "Verificar automaticamente";
                                checked: Prefs.update-check;
                                toggled(on) => { Actions.set-update-check(on); }
                            }
                        }
                        Text {
                            accessible-role: none;
                            accessible-label: "";
                            text: "Verificar automaticamente";
                            font-size: 13.5px;
                            color: Theme.text;
                            vertical-alignment: center;
                        }
                    }
                    Text {
                        text: "Consulta o GitHub uma vez por dia. Nada além disso é enviado.";
                        font-size: 12.5px;
                        color: Theme.sub;
                        wrap: word-wrap;
                    }
                    HorizontalLayout {
                        alignment: start;
                        spacing: 12px;
                        DialogButton { text: "Verificar agora"; clicked => { Actions.check-updates(); } }
                        Text { text: Prefs.update-status; font-size: 13px; color: Theme.sub; vertical-alignment: center; }
                    }
                }
            }
        }
    }
}
```

Note: Slint gives every `Text` an implicit `accessible-label` equal to its text (`lower_accessibility.rs`), and `ElementHandle::find_by_accessible_label` does not filter by role. A `Text` that only repeats its control's label therefore gets `accessible-role: none; accessible-label: "";` — otherwise screen readers read it twice and the tests find two elements.

`crates/app-slint/ui/icons/sliders.svg` (new):

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round"><path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1"/><circle cx="15" cy="6" r="2"/><circle cx="9" cy="12" r="2"/><circle cx="17" cy="18" r="2"/></svg>
```

`crates/app-slint/ui/sidebar.slint`:
- import `IconButton` from `components.slint`.
- In `changed selected => {`, first line: `if (root.selected < 0) { return; }`.
- On the selection highlight `Rectangle` inside `list := Flickable` add `visible: root.selected >= 0;`.
- Footer: the "Novo projeto" `Rectangle` gets `width: parent.width - 20px - 36px;`, and add after it:

```slint
    IconButton {
        x: parent.width - 10px - self.width;
        y: parent.height - 44px;
        icon: @image-url("icons/sliders.svg");
        label: "Configurações";
        clicked => { Actions.open-settings(); }
    }
```

`crates/app-slint/ui/app.slint`:
- imports: `import { SettingsPage } from "settings.slint";`; exports become:

```slint
export { Actions, Picker, Prefs } from "globals.slint";
export { Theme } from "theme.slint";
export { TagChip, NavItem, TaskItem, CalCell, ProjectChoice, HelpItem } from "types.slint";
```

- add properties to `AppWindow`:

```slint
    // 0 = lista de tarefas, 1 = configurações
    in property <int> page;
    in property <bool> help-open;
```

- in `main := Rectangle`, the faded `Rectangle { opacity: ... }` currently holds the list `VerticalLayout`. Wrap that `VerticalLayout` in `Rectangle { visible: root.page == 0; ... }` and add a sibling after it, still inside the faded rectangle:

```slint
                    if root.page == 1 : SettingsPage { }
```

- [ ] **Step 4: Implement the Rust side**

`crates/app-slint/src/bind.rs`:
- imports: `use crate::controller::Page;`, `use ray_core::ThemeMode;`, and add `HelpItem, Prefs, Theme` to the `crate::{...}` import.
- helpers:

```rust
fn theme_int(mode: ThemeMode) -> i32 {
    match mode {
        ThemeMode::System => 0,
        ThemeMode::Light => 1,
        ThemeMode::Dark => 2,
    }
}

fn theme_from_int(n: i32) -> ThemeMode {
    match n {
        1 => ThemeMode::Light,
        2 => ThemeMode::Dark,
        _ => ThemeMode::System,
    }
}
```

- in `bind`, after `wire_keyboard(ui, &shared);` add `wire_settings(ui, &shared);`, and before `refresh(&shared);`:

```rust
    let prefs = ui.global::<Prefs>();
    prefs.set_version(env!("CARGO_PKG_VERSION").into());
    let help: Vec<HelpItem> = keys::help_items().into_iter().map(|(keys, text)| HelpItem { keys: keys.into(), text: text.into() }).collect();
    prefs.set_help_rows(ModelRc::new(VecModel::from(help)));
```

- in `refresh`, replace `ui.set_selected_nav(c.selected_nav() as i32);` with:

```rust
    ui.set_selected_nav(if c.page == Page::Settings { -1 } else { c.selected_nav() as i32 });
    ui.set_page(c.page.as_int());
    ui.set_help_open(c.help_open);
    let settings = c.store.settings();
    ui.global::<Theme>().set_mode(theme_int(settings.theme));
    let prefs = ui.global::<Prefs>();
    prefs.set_theme_mode(theme_int(settings.theme));
    prefs.set_update_check(settings.update_check);
```

- crossfade refactor — replace `switch_view` with:

```rust
/// Crossfade: some (60 ms), troca o conteúdo, reaparece (60 ms).
fn crossfade(s: &Rc<Shared>, change: impl FnOnce(&mut Controller) + 'static) {
    let Some(ui) = s.ui.upgrade() else { return };
    ui.set_content_faded(true);
    let s1 = s.clone();
    Timer::single_shot(Duration::from_millis(60), move || {
        update(&s1, change);
        // Pedido de data pendente da visão anterior não pode abrir o popover mais tarde.
        close_picker(&s1);
        if let Some(ui) = s1.ui.upgrade() {
            ui.set_content_faded(false);
            ui.invoke_focus_root();
        }
    });
}

/// Troca de visão com crossfade. A seleção da sidebar desliza na hora.
pub(crate) fn switch_view(s: &Rc<Shared>, view: View, nav_index: usize) {
    let Some(ui) = s.ui.upgrade() else { return };
    {
        let c = s.ctrl.borrow();
        if c.view == view && c.page == Page::Tasks {
            return;
        }
    }
    ui.set_selected_nav(nav_index as i32);
    crossfade(s, move |c| c.select_view(view));
}
```

- move the `on_open_settings` wiring from Task 4 into `wire_settings` and give it the crossfade; wire the other two callbacks:

```rust
fn wire_settings(ui: &AppWindow, s: &Rc<Shared>) {
    let actions = ui.global::<Actions>();
    {
        let s = s.clone();
        actions.on_open_settings(move || {
            if s.ctrl.borrow().page != Page::Settings {
                crossfade(&s, |c| c.open_settings());
            }
        });
    }
    {
        let s = s.clone();
        actions.on_set_theme(move |n| update(&s, |c| c.set_theme(theme_from_int(n))));
    }
    {
        let s = s.clone();
        actions.on_set_update_check(move |on| update(&s, |c| c.set_update_check(on)));
    }
}
```

(Remove the Task-4 `on_open_settings` block from `wire_keyboard`; keep `on_toggle_help` there.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task`
Expected: all pass.

- [ ] **Step 6: Look at it**

Run: `CARGO_BUILD_JOBS=2 cargo run -p ray-task` — open Settings with the sidebar button and with `Ctrl+,`; switch Sistema/Claro/Escuro; restart and confirm the theme sticks; `Esc` returns to the list.

- [ ] **Step 7: Commit**

```bash
git add crates/app-slint
git commit -F - <<'EOF'
feat(app): add a settings page with light, dark and system theme

Opened from a sidebar button or Ctrl+,. The saved theme is applied
before the window shows, so there is no flash of the wrong colors.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 6: Shortcut overlay and "Atalhos" section

**Files:**
- Create: `crates/app-slint/ui/help.slint`
- Modify: `crates/app-slint/ui/settings.slint`, `crates/app-slint/ui/app.slint`
- Test: `crates/app-slint/tests/ui_help.rs`

**Interfaces:**
- Consumes: `Prefs.help-rows`, `AppWindow.help-open`, `Actions.toggle-help` (Tasks 4–5).
- Produces: Slint components `ShortcutTable`, `HelpOverlay`.

- [ ] **Step 1: Write the failing test**

`crates/app-slint/tests/ui_help.rs`:

```rust
use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use ray_core::{FixedClock, Snapshot, Store};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::AppWindow;
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, SharedString};

fn ms(n: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(n));
}

fn tap(ui: &AppWindow, key: impl Into<SharedString>) {
    let key: SharedString = key.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: key });
    ms(20);
}

fn ctrl(ui: &AppWindow, key: impl Into<SharedString>) {
    let control: SharedString = Key::Control.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    tap(ui, key);
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: control });
}

fn setup() -> (AppWindow, bind::Binding) {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    ui.show().unwrap();
    ui.invoke_focus_root();
    ms(50);
    (ui, binding)
}

#[test]
fn f1_and_question_mark_open_the_overlay_and_esc_closes_it() {
    let (ui, _binding) = setup();
    tap(&ui, Key::F1);
    assert!(ui.get_help_open());
    assert!(ElementHandle::find_by_accessible_label(&ui, "Atalhos").next().is_some());
    tap(&ui, Key::Escape);
    assert!(!ui.get_help_open());
    tap(&ui, "?");
    assert!(ui.get_help_open());
    tap(&ui, "?");
    assert!(!ui.get_help_open(), "? de novo fecha");
}

#[test]
fn question_mark_in_a_text_field_is_just_text() {
    let (ui, binding) = setup();
    ctrl(&ui, "f");
    ms(50);
    tap(&ui, "?");
    assert!(!ui.get_help_open());
    assert_eq!(binding.controller().filter, "?");
}

#[test]
fn settings_page_lists_the_shortcuts() {
    let (ui, _binding) = setup();
    ctrl(&ui, ",");
    ms(100);
    let rows: Vec<_> = ElementHandle::find_by_accessible_label(&ui, "Ctrl+N").collect();
    assert_eq!(rows.len(), 1, "linha Ctrl+N na seção Atalhos");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test ui_help`
Expected: FAIL — no element labelled "Atalhos" / "Ctrl+N".

- [ ] **Step 3: Implement**

`crates/app-slint/ui/help.slint` (new):

```slint
import { Theme } from "theme.slint";
import { Actions, Prefs } from "globals.slint";

/// Tabela de atalhos (fonte: keys::HELP no Rust).
export component ShortcutTable inherits VerticalLayout {
    spacing: 8px;
    for row in Prefs.help-rows : HorizontalLayout {
        spacing: 16px;
        Text {
            text: row.keys;
            width: 150px;
            font-size: 13px;
            font-weight: 600;
            color: Theme.text;
            accessible-role: text;
            accessible-label: row.keys;
        }
        Text { text: row.text; font-size: 13px; color: Theme.sub; wrap: word-wrap; horizontal-stretch: 1; }
    }
}

/// Overlay de atalhos (F1 / ?): cartão central; Esc ou clique fora fecha.
export component HelpOverlay inherits Rectangle {
    background: #00000040;
    TouchArea { clicked => { Actions.toggle-help(); } }
    Rectangle {
        width: min(520px, root.width - 48px);
        height: min(card.preferred-height, root.height - 48px);
        background: Theme.pop;
        border-radius: 14px;
        drop-shadow-blur: 32px;
        drop-shadow-offset-y: 12px;
        drop-shadow-color: #0000002e;
        clip: true;
        accessible-role: groupbox;
        accessible-label: "Atalhos";
        // Clique no cartão não fecha.
        TouchArea { }
        card := VerticalLayout {
            padding: 20px;
            spacing: 14px;
            HorizontalLayout {
                Text { text: "Atalhos"; font-size: 17px; font-weight: 700; color: Theme.text; }
                Rectangle { horizontal-stretch: 1; }
                Text { text: "Esc fecha"; font-size: 12px; color: Theme.faint; vertical-alignment: center; }
            }
            ShortcutTable { }
        }
    }
}
```

`crates/app-slint/ui/settings.slint` — import `ShortcutTable` from `help.slint` and add a third section after "Atualizações":

```slint
                Section {
                    title: "Atalhos";
                    ShortcutTable { }
                }
```

`crates/app-slint/ui/app.slint` — import `HelpOverlay` from `help.slint` and, inside `keys := FocusScope`, after the `confirm-visible` dialog:

```slint
        if root.help-open : HelpOverlay { }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task`
Expected: all pass. If `settings_page_lists_the_shortcuts` finds 0 rows because the Settings page is scrolled/virtualised, it is not (plain `for` in a `Flickable` instantiates every row) — check that `SettingsPage` is mounted (`ui.get_page() == 1`).

- [ ] **Step 5: Commit**

```bash
git add crates/app-slint
git commit -F - <<'EOF'
feat(app): show the shortcut list on F1 or ? and in settings

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 7: Pure update logic

**Files:**
- Create: `crates/app-slint/src/update.rs`, `crates/app-slint/tests/fixtures/github-release.json`
- Modify: `crates/app-slint/src/lib.rs`, `crates/app-slint/Cargo.toml`

**Interfaces:**
- Consumes: `ray_core::Settings` (Task 2).
- Produces (in `ray_task::update`):
  - `pub struct Release { pub version: semver::Version, pub url: String, pub notes: String }` (`Debug, Clone, PartialEq`)
  - `pub enum UpdateError { Network(String), Invalid(String), ForeignUrl(String) }` (`thiserror`, `Display` in Portuguese)
  - `pub fn parse_release(json: &str) -> Result<Release, UpdateError>`
  - `pub fn is_newer(current: &Version, candidate: &Version, dismissed: Option<&str>) -> bool`
  - `pub fn should_check(settings: &Settings, now: DateTime<Utc>) -> bool`
  - `pub fn notes_excerpt(body: &str) -> String`
  - `pub fn install_command_for(windows: bool) -> &'static str`, `pub fn install_command() -> &'static str`
  - `pub fn current_version() -> Version`
  - consts `RELEASES_URL_PREFIX`, `FIRST_CHECK_DELAY: std::time::Duration` (5 s), `RECHECK_EVERY: std::time::Duration` (6 h)

- [ ] **Step 1: Add dependencies**

`crates/app-slint/Cargo.toml` `[dependencies]`:

```toml
semver = "1"
serde_json = "1"
thiserror.workspace = true
```

- [ ] **Step 2: Write the fixture and failing tests**

`crates/app-slint/tests/fixtures/github-release.json` (shape of the real API response; body follows what `dist` publishes):

```json
{
  "url": "https://api.github.com/repos/carvalhosauro/ray-task/releases/1",
  "html_url": "https://github.com/carvalhosauro/ray-task/releases/tag/v0.2.0",
  "tag_name": "v0.2.0",
  "name": "0.2.0",
  "draft": false,
  "prerelease": false,
  "body": "## Release Notes\n\n### Features\n\n- add a trash button to task rows\n- settings page with light and dark theme\n\n## Install ray-task 0.2.0\n\n### Install prebuilt binaries via shell script\n\n```sh\ncurl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/download/v0.2.0/ray-task-installer.sh | sh\n```\n"
}
```

`crates/app-slint/src/update.rs` — start with the tests module at the bottom:

```rust
#[cfg(test)]
mod tests {
    use chrono::{NaiveDateTime, TimeDelta};

    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/github-release.json");

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn at(s: &str) -> DateTime<Utc> {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap().and_utc()
    }

    #[test]
    fn parses_the_github_release() {
        let r = parse_release(FIXTURE).unwrap();
        assert_eq!(r.version, v("0.2.0"));
        assert_eq!(r.url, "https://github.com/carvalhosauro/ray-task/releases/tag/v0.2.0");
        assert_eq!(r.notes, "• add a trash button to task rows\n• settings page with light and dark theme");
    }

    #[test]
    fn rejects_urls_outside_the_repository() {
        let json = FIXTURE.replace("https://github.com/carvalhosauro/ray-task/releases/tag/v0.2.0", "https://evil.example/x");
        assert!(matches!(parse_release(&json), Err(UpdateError::ForeignUrl(_))));
    }

    #[test]
    fn rejects_bad_json_and_bad_tags() {
        assert!(matches!(parse_release("not json"), Err(UpdateError::Invalid(_))));
        assert!(matches!(parse_release("{}"), Err(UpdateError::Invalid(_))));
        let json = FIXTURE.replace("\"v0.2.0\"", "\"nightly\"");
        assert!(matches!(parse_release(&json), Err(UpdateError::Invalid(_))));
    }

    #[test]
    fn is_newer_compares_semver() {
        assert!(is_newer(&v("0.1.0"), &v("0.2.0"), None));
        assert!(is_newer(&v("0.9.0"), &v("0.10.0"), None), "semver, não texto");
        assert!(!is_newer(&v("0.2.0"), &v("0.2.0"), None));
        assert!(!is_newer(&v("0.2.0"), &v("0.1.9"), None));
    }

    #[test]
    fn is_newer_respects_the_dismissed_version() {
        assert!(!is_newer(&v("0.1.0"), &v("0.2.0"), Some("0.2.0")));
        assert!(is_newer(&v("0.1.0"), &v("0.3.0"), Some("0.2.0")), "versão maior volta a avisar");
        assert!(!is_newer(&v("0.1.0"), &v("0.2.0"), Some("0.3.0")));
        assert!(is_newer(&v("0.1.0"), &v("0.2.0"), Some("lixo")), "dispensada ilegível não esconde");
    }

    #[test]
    fn should_check_once_a_day_when_enabled() {
        let mut s = Settings::default();
        assert!(should_check(&s, at("2026-10-06 10:00")), "nunca verificou");
        s.update_last_check = Some(at("2026-10-06 10:00"));
        assert!(!should_check(&s, at("2026-10-07 09:59")));
        assert!(should_check(&s, at("2026-10-07 10:00")));
        s.update_check = false;
        assert!(!should_check(&s, at("2026-10-09 10:00")));
    }

    #[test]
    fn should_check_when_last_check_is_in_the_future() {
        let s = Settings { update_last_check: Some(at("2026-10-06 10:00") + TimeDelta::days(30)), ..Settings::default() };
        assert!(should_check(&s, at("2026-10-06 10:00")), "relógio voltou: não trava para sempre");
    }

    #[test]
    fn excerpt_keeps_only_the_changelog() {
        let dist_only = "## Install ray-task 0.1.0\n\n### Install prebuilt binaries via shell script\n\n```sh\ncurl x | sh\n```\n";
        assert_eq!(notes_excerpt(dist_only), "");
        let long = format!("- {}\n", "a".repeat(100));
        assert_eq!(notes_excerpt(&long), format!("• {}…", "a".repeat(78)));
        let many = (1..=8).map(|i| format!("* item {i}")).collect::<Vec<_>>().join("\n");
        assert_eq!(notes_excerpt(&many).lines().count(), 5);
        assert_eq!(notes_excerpt("Plain line\n\n# Title\n"), "Plain line");
    }

    #[test]
    fn install_commands_match_the_readme() {
        assert_eq!(
            install_command_for(false),
            "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.sh | sh"
        );
        assert_eq!(
            install_command_for(true),
            "powershell -ExecutionPolicy Bypass -c \"irm https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.ps1 | iex\""
        );
        let readme = include_str!("../../../README.md");
        assert!(readme.contains(install_command_for(false)));
        assert!(readme.contains(install_command_for(true)));
    }
}
```

Add `pub mod update;` to `crates/app-slint/src/lib.rs`.

- [ ] **Step 3: Run tests to verify they fail**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --lib update`
Expected: compile errors (functions not defined).

- [ ] **Step 4: Implement** (top of `crates/app-slint/src/update.rs`)

```rust
//! Aviso de versão nova: lógica pura (JSON do GitHub, versões, quando verificar) e a busca HTTP.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use ray_core::Settings;
use semver::Version;

/// Só URLs deste prefixo são abertas no navegador.
pub const RELEASES_URL_PREFIX: &str = "https://github.com/carvalhosauro/ray-task/releases/";
/// Primeira verificação depois de abrir (não pesa na inicialização).
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(5);
/// O app pode ficar aberto por dias: reavalia a cada 6 h (a verificação em si é diária).
pub const RECHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const CHECK_INTERVAL_HOURS: i64 = 24;
const EXCERPT_LINES: usize = 5;
const EXCERPT_WIDTH: usize = 80;

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: Version,
    pub url: String,
    /// Resumo do changelog (pode ser vazio).
    pub notes: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("rede: {0}")]
    Network(String),
    #[error("resposta inválida: {0}")]
    Invalid(String),
    #[error("URL fora do repositório: {0}")]
    ForeignUrl(String),
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("versão do Cargo.toml é semver")
}

pub fn parse_release(json: &str) -> Result<Release, UpdateError> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| UpdateError::Invalid(e.to_string()))?;
    let tag = value["tag_name"].as_str().ok_or_else(|| UpdateError::Invalid("sem tag_name".into()))?;
    let version = Version::parse(tag.trim_start_matches('v')).map_err(|e| UpdateError::Invalid(format!("tag '{tag}': {e}")))?;
    let url = value["html_url"].as_str().ok_or_else(|| UpdateError::Invalid("sem html_url".into()))?;
    if !url.starts_with(RELEASES_URL_PREFIX) {
        return Err(UpdateError::ForeignUrl(url.to_string()));
    }
    let notes = notes_excerpt(value["body"].as_str().unwrap_or_default());
    Ok(Release { version, url: url.to_string(), notes })
}

/// `candidate` é maior que a versão atual e que a versão dispensada (se legível).
pub fn is_newer(current: &Version, candidate: &Version, dismissed: Option<&str>) -> bool {
    candidate > current && dismissed.and_then(|d| Version::parse(d).ok()).is_none_or(|d| candidate > &d)
}

/// Verificação automática: ligada e (nunca verificou, passou um dia, ou o relógio voltou).
pub fn should_check(settings: &Settings, now: DateTime<Utc>) -> bool {
    settings.update_check
        && settings.update_last_check.is_none_or(|last| last > now || now - last >= TimeDelta::hours(CHECK_INTERVAL_HOURS))
}

/// O corpo do release publicado pelo `dist` é o changelog seguido de `## Install …`: lê só o que
/// vem antes, sem títulos nem blocos de código; marcadores viram `• `.
pub fn notes_excerpt(body: &str) -> String {
    let mut in_code = false;
    let mut lines = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if line.starts_with("## Install") {
            break;
        }
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let text = match line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            Some(item) => format!("• {}", item.trim()),
            None => line.to_string(),
        };
        lines.push(cut(&text));
        if lines.len() == EXCERPT_LINES {
            break;
        }
    }
    lines.join("\n")
}

fn cut(text: &str) -> String {
    if text.chars().count() <= EXCERPT_WIDTH {
        return text.to_string();
    }
    let mut out: String = text.chars().take(EXCERPT_WIDTH - 1).collect();
    out.push('…');
    out
}

pub fn install_command_for(windows: bool) -> &'static str {
    if windows {
        "powershell -ExecutionPolicy Bypass -c \"irm https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.ps1 | iex\""
    } else {
        "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.sh | sh"
    }
}

pub fn install_command() -> &'static str {
    install_command_for(cfg!(windows))
}
```

(`Option::is_none_or` is stable since Rust 1.82; the MSRV is 1.92.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --lib update`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/app-slint/Cargo.toml Cargo.lock crates/app-slint/src/lib.rs crates/app-slint/src/update.rs crates/app-slint/tests/fixtures
git commit -F - <<'EOF'
feat(app): parse GitHub releases and decide when to check for updates

Pure logic only: release JSON, semver comparison with a dismissed
version, the once-a-day window and a short changelog excerpt. URLs
outside the project's releases page are rejected.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 8: Update-check state in the controller

**Files:**
- Modify: `crates/app-slint/src/controller.rs`
- Test: `crates/app-slint/tests/controller.rs`

**Interfaces:**
- Consumes: `update::{Release, UpdateError, is_newer, should_check, current_version}` (Task 7), `Store::{mark_update_checked, dismiss_update, now_utc}` (Task 2).
- Produces:
  - `pub enum CheckStatus { Idle, Checking, UpToDate, Available, Failed }` (`Debug, Clone, Copy, PartialEq, Eq`)
  - `Controller` fields `pub current_version: semver::Version` (init `update::current_version()`), `pub check: CheckStatus` (init `Idle`)
  - `Controller::begin_check(&mut self, manual: bool, now: DateTime<Utc>) -> bool`
  - `Controller::finish_check(&mut self, result: Result<Release, UpdateError>)`
  - `Controller::update_notice(&self) -> Option<&Release>`
  - `Controller::dismiss_update(&mut self)`
  - `Controller::check_status_text(&self) -> String`

- [ ] **Step 1: Write the failing tests**

Append to `crates/app-slint/tests/controller.rs` (add `use ray_task::controller::CheckStatus;`, `use ray_task::update::{Release, UpdateError};`, `use semver::Version;`, `use chrono::{DateTime, Utc};`):

```rust
fn release(version: &str) -> Release {
    Release {
        version: Version::parse(version).unwrap(),
        url: format!("https://github.com/carvalhosauro/ray-task/releases/tag/v{version}"),
        notes: String::new(),
    }
}

fn utc(s: &str) -> DateTime<Utc> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap().and_utc()
}

fn on_v010() -> Fixture {
    let mut f = at("2026-10-06 10:00");
    f.c.current_version = Version::new(0, 1, 0);
    f
}

#[test]
fn successful_check_shows_a_newer_release_and_records_the_time() {
    let mut f = on_v010();
    assert!(f.c.begin_check(false, utc("2026-10-06 10:00")));
    assert_eq!(f.c.check, CheckStatus::Checking);
    f.c.finish_check(Ok(release("0.2.0")));
    assert_eq!(f.c.check, CheckStatus::Available);
    assert_eq!(f.c.update_notice().map(|r| r.version.to_string()), Some("0.2.0".into()));
    assert_eq!(f.c.check_status_text(), "Versão 0.2.0 disponível");
    assert!(f.c.store.settings().update_last_check.is_some());
    assert!(f.sent.borrow().iter().any(|op| matches!(op, WriteOp::SetSetting { key: "update_last_check", .. })));
}

#[test]
fn same_version_is_up_to_date() {
    let mut f = on_v010();
    assert!(f.c.begin_check(true, utc("2026-10-06 10:00")));
    f.c.finish_check(Ok(release("0.1.0")));
    assert_eq!(f.c.check, CheckStatus::UpToDate);
    assert!(f.c.update_notice().is_none());
    assert_eq!(f.c.check_status_text(), "Você está na versão mais recente");
}

#[test]
fn failures_are_silent_for_automatic_checks_and_visible_for_manual_ones() {
    let mut f = on_v010();
    assert!(f.c.begin_check(false, utc("2026-10-06 10:00")));
    f.c.finish_check(Err(UpdateError::Network("offline".into())));
    assert_eq!(f.c.check, CheckStatus::Idle);
    assert_eq!(f.c.check_status_text(), "");
    assert!(f.c.store.settings().update_last_check.is_none(), "falha não conta como verificado");

    assert!(f.c.begin_check(true, utc("2026-10-06 10:00")));
    f.c.finish_check(Err(UpdateError::Network("offline".into())));
    assert_eq!(f.c.check, CheckStatus::Failed);
    assert_eq!(f.c.check_status_text(), "Não foi possível verificar");
}

#[test]
fn one_check_at_a_time_and_stale_responses_are_ignored() {
    let mut f = on_v010();
    assert!(f.c.begin_check(false, utc("2026-10-06 10:00")));
    assert!(!f.c.begin_check(true, utc("2026-10-06 10:00")), "já há uma em andamento");
    f.c.finish_check(Ok(release("0.1.0")));
    f.c.finish_check(Ok(release("0.2.0")));
    assert!(f.c.update_notice().is_none(), "resposta sem verificação em andamento é ignorada");
}

#[test]
fn automatic_check_respects_the_daily_window_and_the_toggle() {
    let mut f = on_v010();
    assert!(f.c.begin_check(false, utc("2026-10-06 10:00")));
    f.c.finish_check(Ok(release("0.1.0")));
    assert!(!f.c.begin_check(false, utc("2026-10-06 16:00")), "menos de 24 h");
    assert!(f.c.begin_check(true, utc("2026-10-06 16:00")), "manual ignora a janela");
    f.c.finish_check(Ok(release("0.1.0")));
    f.c.set_update_check(false);
    assert!(!f.c.begin_check(false, utc("2026-10-09 10:00")));
}

#[test]
fn dismissing_hides_the_notice_until_a_greater_version() {
    let mut f = on_v010();
    f.c.begin_check(true, utc("2026-10-06 10:00"));
    f.c.finish_check(Ok(release("0.2.0")));
    f.c.dismiss_update();
    assert!(f.c.update_notice().is_none());
    assert_eq!(f.c.store.settings().update_dismissed.as_deref(), Some("0.2.0"));
    f.c.begin_check(true, utc("2026-10-07 10:00"));
    f.c.finish_check(Ok(release("0.2.0")));
    assert!(f.c.update_notice().is_none());
    f.c.begin_check(true, utc("2026-10-08 10:00"));
    f.c.finish_check(Ok(release("0.3.0")));
    assert_eq!(f.c.update_notice().map(|r| r.version.to_string()), Some("0.3.0".into()));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test controller`
Expected: compile errors (`CheckStatus`, `begin_check` not found).

- [ ] **Step 3: Implement**

In `crates/app-slint/src/controller.rs` (import `chrono::{DateTime, Utc}`, `semver::Version`, `crate::update::{self, Release, UpdateError}`):

```rust
/// Verificação de atualização.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Idle,
    Checking,
    UpToDate,
    Available,
    /// Só para verificação manual; a automática falha em silêncio.
    Failed,
}
```

Fields on `Controller`:

```rust
    pub current_version: Version,
    pub check: CheckStatus,
    check_manual: bool,
    release: Option<Release>,
```

initialised as `current_version: update::current_version(), check: CheckStatus::Idle, check_manual: false, release: None,`.

Methods:

```rust
    /// Começa uma verificação. Automática só se `should_check`; nunca duas ao mesmo tempo.
    pub fn begin_check(&mut self, manual: bool, now: DateTime<Utc>) -> bool {
        if self.check == CheckStatus::Checking || (!manual && !update::should_check(self.store.settings(), now)) {
            return false;
        }
        self.check = CheckStatus::Checking;
        self.check_manual = manual;
        true
    }

    pub fn finish_check(&mut self, result: Result<Release, UpdateError>) {
        if self.check != CheckStatus::Checking {
            return; // resposta atrasada ou repetida
        }
        match result {
            Ok(release) => {
                self.store.mark_update_checked();
                self.release = Some(release);
                self.check = if self.update_notice().is_some() { CheckStatus::Available } else { CheckStatus::UpToDate };
                self.flush();
            }
            Err(error) => {
                tracing::warn!(%error, "verificar atualização");
                self.check = if self.check_manual { CheckStatus::Failed } else { CheckStatus::Idle };
            }
        }
    }

    /// Versão nova a avisar (maior que a atual e que a dispensada).
    pub fn update_notice(&self) -> Option<&Release> {
        let dismissed = self.store.settings().update_dismissed.as_deref();
        self.release.as_ref().filter(|r| update::is_newer(&self.current_version, &r.version, dismissed))
    }

    pub fn dismiss_update(&mut self) {
        let Some(version) = self.update_notice().map(|r| r.version.to_string()) else { return };
        self.store.dismiss_update(version);
        if self.check == CheckStatus::Available {
            self.check = CheckStatus::Idle;
        }
        self.flush();
    }

    pub fn check_status_text(&self) -> String {
        match self.check {
            CheckStatus::Idle => String::new(),
            CheckStatus::Checking => "Verificando…".into(),
            CheckStatus::UpToDate => "Você está na versão mais recente".into(),
            CheckStatus::Available => {
                self.update_notice().map(|r| format!("Versão {} disponível", r.version)).unwrap_or_default()
            }
            CheckStatus::Failed => "Não foi possível verificar".into(),
        }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test controller`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/app-slint/src/controller.rs crates/app-slint/tests/controller.rs
git commit -F - <<'EOF'
feat(app): keep update-check state in the controller

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 9: Fetch releases in the background and wire the check

**Files:**
- Modify: `crates/app-slint/Cargo.toml`, `crates/app-slint/src/update.rs`, `crates/app-slint/src/bind.rs`, `crates/app-slint/src/main.rs`, `crates/app-slint/ui/globals.slint`
- Test: `crates/app-slint/tests/ui_update.rs`

**Interfaces:**
- Consumes: Task 8 controller API; `Prefs.update-status` (Task 5).
- Produces:
  - `update::fetch() -> Result<String, UpdateError>`
  - `pub type Fetch = std::sync::Arc<dyn Fn() -> Result<String, UpdateError> + Send + Sync>` in `bind.rs`
  - `Binding::start_update_checks(&mut self, fetch: Fetch)`
  - Slint `Actions.update-response(bool, string)`; `Actions.check-updates()` now runs a manual check.

- [ ] **Step 1: Write the failing test**

`crates/app-slint/tests/ui_update.rs`:

```rust
use ray_core::{FixedClock, Snapshot, Store};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow, Prefs};
use semver::Version;
use slint::ComponentHandle;

const FIXTURE: &str = include_str!("fixtures/github-release.json");

fn setup() -> (AppWindow, bind::Binding) {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-06 10:00")));
    let mut ctrl = Controller::new(store, Box::new(|_| {}));
    ctrl.current_version = Version::new(0, 1, 0);
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, ctrl, Box::new(|| {}));
    (ui, binding)
}

#[test]
fn manual_check_shows_status_and_notice() {
    let (ui, _binding) = setup();
    let actions = ui.global::<Actions>();
    let prefs = ui.global::<Prefs>();

    actions.invoke_check_updates();
    assert_eq!(prefs.get_update_status(), "Verificando…");
    actions.invoke_update_response(true, FIXTURE.into());
    assert_eq!(prefs.get_update_status(), "Versão 0.2.0 disponível");
    assert_eq!(prefs.get_notice_version(), "0.2.0");
    assert_eq!(prefs.get_notice_notes(), "• add a trash button to task rows\n• settings page with light and dark theme");
}

#[test]
fn manual_check_failure_is_shown() {
    let (ui, _binding) = setup();
    let actions = ui.global::<Actions>();
    actions.invoke_check_updates();
    actions.invoke_update_response(false, "timeout".into());
    assert_eq!(ui.global::<Prefs>().get_update_status(), "Não foi possível verificar");
    assert_eq!(ui.global::<Prefs>().get_notice_version(), "");
}

#[test]
fn foreign_url_is_treated_as_a_failure() {
    let (ui, _binding) = setup();
    let actions = ui.global::<Actions>();
    actions.invoke_check_updates();
    let json = FIXTURE.replace("https://github.com/carvalhosauro/ray-task/releases/tag/v0.2.0", "https://evil.example/x");
    actions.invoke_update_response(true, json.into());
    assert_eq!(ui.global::<Prefs>().get_notice_version(), "");
    assert_eq!(ui.global::<Prefs>().get_update_status(), "Não foi possível verificar");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test ui_update`
Expected: compile error (`invoke_update_response` not found).

- [ ] **Step 3: Add the network shell**

`crates/app-slint/Cargo.toml`:

```toml
ureq = "3"
open = "5"
```

Append to `crates/app-slint/src/update.rs` (above the tests module):

```rust
const API_URL: &str = "https://api.github.com/repos/carvalhosauro/ray-task/releases/latest";
const TIMEOUT: Duration = Duration::from_secs(10);

/// GET do último release (bloqueia: chame fora do event loop). Devolve o corpo cru.
pub fn fetch() -> Result<String, UpdateError> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(TIMEOUT)).build().into();
    let mut response = agent
        .get(API_URL)
        .header("User-Agent", concat!("ray-task/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    response.body_mut().read_to_string().map_err(|e| UpdateError::Network(e.to_string()))
}
```

(If the `ureq` 3 API differs in the resolved version, check https://docs.rs/ureq/3 — the shape is `Agent::config_builder()…build().into()`, `.get(url).header(k, v).call()`, `response.body_mut().read_to_string()`; non-2xx statuses are returned as `Err` by default.)

- [ ] **Step 4: Wire it in Slint and bind**

`crates/app-slint/ui/globals.slint` — in `Actions`, under `check-updates`:

```slint
    // resposta da busca (thread de rede): ok + corpo JSON, ou erro
    callback update-response(bool, string);
```

`crates/app-slint/src/bind.rs`:
- add `use std::sync::Arc;` and `use crate::update::{self, UpdateError};`
- `pub type Fetch = Arc<dyn Fn() -> Result<String, UpdateError> + Send + Sync>;`
- `Shared` gets `fetch: RefCell<Option<Fetch>>,` (init `RefCell::new(None)`).
- in `refresh`, after the `prefs` lines from Task 5:

```rust
    prefs.set_update_status(c.check_status_text().into());
    match c.update_notice() {
        Some(release) => {
            prefs.set_notice_version(release.version.to_string().into());
            prefs.set_notice_notes(release.notes.clone().into());
        }
        None => {
            prefs.set_notice_version("".into());
            prefs.set_notice_notes("".into());
        }
    }
```

- in `bind`, next to `set_version`: `prefs.set_install_command(update::install_command().into());`
- functions:

```rust
/// Começa uma verificação (se o controller deixar) e busca numa thread; a resposta volta pelo
/// callback `update-response`, no event loop.
fn run_check(s: &Rc<Shared>, manual: bool) {
    let started = {
        let mut c = s.ctrl.borrow_mut();
        let now = c.store.now_utc();
        c.begin_check(manual, now)
    };
    if !started {
        return;
    }
    refresh(s);
    let Some(fetch) = s.fetch.borrow().clone() else { return };
    let weak = s.ui.clone();
    std::thread::spawn(move || {
        let (ok, body) = match fetch() {
            Ok(body) => (true, body),
            Err(error) => (false, error.to_string()),
        };
        let _ = weak.upgrade_in_event_loop(move |ui| ui.global::<Actions>().invoke_update_response(ok, body.into()));
    });
}
```

- in `wire_settings`:

```rust
    {
        let s = s.clone();
        actions.on_check_updates(move || run_check(&s, true));
    }
    {
        let s = s.clone();
        actions.on_update_response(move |ok, body| {
            let result = if ok { update::parse_release(&body) } else { Err(UpdateError::Network(body.to_string())) };
            update(&s, |c| c.finish_check(result));
        });
    }
```

- `impl Binding`:

```rust
    /// Liga a verificação automática: 5 s depois de abrir e a cada 6 h (o controller decide se
    /// já passou um dia).
    pub fn start_update_checks(&mut self, fetch: Fetch) {
        *self.shared.fetch.borrow_mut() = Some(fetch);
        let s = self.shared.clone();
        Timer::single_shot(update::FIRST_CHECK_DELAY, move || run_check(&s, false));
        let timer = Rc::new(Timer::default());
        let s = self.shared.clone();
        timer.start(TimerMode::Repeated, update::RECHECK_EVERY, move || run_check(&s, false));
        self._timers.push(timer);
    }
```

`crates/app-slint/src/main.rs`:

```rust
    let mut binding = bind::bind(&ui, controller, Box::new(move || retry.retry()));
    binding.start_update_checks(std::sync::Arc::new(ray_task::update::fetch));
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task`
Expected: all pass.

- [ ] **Step 6: Try it for real**

Run: `CARGO_BUILD_JOBS=2 cargo run -p ray-task`, open Settings, click "Verificar agora". With network: "Você está na versão mais recente" (0.1.0 is the latest). Disconnect and click again: "Não foi possível verificar".

- [ ] **Step 7: Commit**

```bash
git add crates/app-slint Cargo.lock
git commit -F - <<'EOF'
feat(app): check GitHub for new releases in the background

One GET a day at most, 5 s after startup and re-evaluated every 6 h.
The request runs on its own thread and hands the body back to the event
loop; automatic failures are only logged.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 10: Sidebar update notice and popover

**Files:**
- Modify: `crates/app-slint/ui/sidebar.slint`, `crates/app-slint/ui/globals.slint`, `crates/app-slint/src/bind.rs`
- Test: `crates/app-slint/tests/ui_update.rs`

**Interfaces:**
- Consumes: `Prefs.notice-version`, `Prefs.notice-notes`, `Prefs.install-command` (Tasks 5, 9); `Controller::dismiss_update`, `update_notice` (Task 8).
- Produces: Slint `Actions.open-release()`, `Actions.dismiss-update()`.

- [ ] **Step 1: Write the failing test**

Append to `crates/app-slint/tests/ui_update.rs` (add `use i_slint_backend_testing::ElementHandle;`):

```rust
#[test]
fn notice_appears_in_the_sidebar_and_can_be_dismissed() {
    let (ui, binding) = setup();
    ui.show().unwrap();
    let actions = ui.global::<Actions>();
    assert_eq!(ElementHandle::find_by_accessible_label(&ui, "v0.2.0 disponível").count(), 0);

    actions.invoke_check_updates();
    actions.invoke_update_response(true, FIXTURE.into());
    assert_eq!(ElementHandle::find_by_accessible_label(&ui, "v0.2.0 disponível").count(), 1);

    actions.invoke_dismiss_update();
    assert_eq!(ui.global::<Prefs>().get_notice_version(), "");
    assert_eq!(ElementHandle::find_by_accessible_label(&ui, "v0.2.0 disponível").count(), 0);
    assert_eq!(binding.controller().store.settings().update_dismissed.as_deref(), Some("0.2.0"));
}

#[test]
fn install_command_is_exposed_to_the_ui() {
    let (ui, _binding) = setup();
    assert_eq!(ui.global::<Prefs>().get_install_command(), ray_task::update::install_command());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task --test ui_update`
Expected: compile error (`invoke_dismiss_update` not found).

- [ ] **Step 3: Implement**

`crates/app-slint/ui/globals.slint` — in `Actions`, under `update-response`:

```slint
    callback open-release();
    callback dismiss-update();
```

`crates/app-slint/ui/sidebar.slint`:
- import `Prefs` from `globals.slint` and `DialogButton` from `components.slint`.
- add `property <bool> copied;` to `Sidebar`.
- the project list leaves room for the notice: `list := Flickable { height: parent.height - 52px - 48px - (Prefs.notice-version != "" ? 30px : 0px); ... }`
- after the settings `IconButton`, add:

```slint
    // Cópia do comando de instalação: o Slint não tem API de clipboard, o TextInput tem.
    clip := TextInput {
        width: 0px;
        height: 0px;
        read-only: true;
        text: Prefs.install-command;
    }

    // Aviso discreto de versão nova, acima de "Novo projeto".
    if Prefs.notice-version != "" : Rectangle {
        property <bool> shown;
        init => { self.shown = true; }
        x: 10px;
        y: parent.height - 44px - 30px;
        width: parent.width - 20px;
        height: 26px;
        border-radius: 7px;
        opacity: self.shown ? 1 : 0;
        animate opacity { duration: 300ms; easing: ease-out; }
        background: nt.has-hover ? Theme.selection.with-alpha(0.5) : transparent;
        accessible-role: button;
        accessible-label: "v" + Prefs.notice-version + " disponível";
        accessible-action-default => { update-popup.show(); }
        nt := TouchArea { clicked => { update-popup.show(); } }
        HorizontalLayout {
            padding-left: 12px;
            spacing: 8px;
            VerticalLayout {
                alignment: center;
                Rectangle { width: 6px; height: 6px; border-radius: 3px; background: Theme.accent; }
            }
            Text {
                accessible-role: none;
                accessible-label: "";
                text: "v" + Prefs.notice-version + " disponível";
                font-size: 12px;
                color: Theme.sub;
                vertical-alignment: center;
            }
        }
    }

    update-popup := PopupWindow {
        x: 10px;
        y: root.height - 44px - 34px - self.height;
        width: 280px;
        close-policy: PopupClosePolicy.close-on-click-outside;
        MenuPanel {
            VerticalLayout {
                padding: 14px;
                spacing: 10px;
                Text { text: "ray-task " + Prefs.notice-version; font-size: 14px; font-weight: 600; color: Theme.text; }
                if Prefs.notice-notes != "" : Text { text: Prefs.notice-notes; font-size: 12.5px; color: Theme.sub; wrap: word-wrap; }
                HorizontalLayout {
                    spacing: 8px;
                    DialogButton {
                        text: root.copied ? "Copiado ✓" : "Copiar comando";
                        primary: true;
                        clicked => {
                            clip.select-all();
                            clip.copy();
                            root.copied = true;
                        }
                    }
                    DialogButton { text: "Ver release"; clicked => { Actions.open-release(); } }
                }
                Text {
                    text: "Não avisar desta versão";
                    font-size: 12px;
                    color: Theme.faint;
                    accessible-role: button;
                    accessible-label: self.text;
                    TouchArea {
                        clicked => {
                            Actions.dismiss-update();
                            update-popup.close();
                        }
                    }
                }
            }
        }
    }

    Timer {
        interval: 2s;
        running: root.copied;
        triggered => { root.copied = false; }
    }
```

`crates/app-slint/src/bind.rs` — in `wire_settings`:

```rust
    {
        let s = s.clone();
        actions.on_open_release(move || {
            let url = s.ctrl.borrow().update_notice().map(|r| r.url.clone());
            if let Some(url) = url {
                // URL já validada em update::parse_release (só a página de releases do projeto).
                if let Err(error) = open::that_detached(&url) {
                    tracing::warn!(%error, %url, "abrir release no navegador");
                }
            }
        });
    }
    {
        let s = s.clone();
        actions.on_dismiss_update(move || update(&s, |c| c.dismiss_update()));
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_BUILD_JOBS=2 cargo test -p ray-task`
Expected: all pass.

- [ ] **Step 5: Try it for real (copy and browser can only be checked by hand)**

Temporarily set `current_version` lower to see the notice: run `CARGO_BUILD_JOBS=2 cargo run -p ray-task` after changing `version = "0.1.0"` to `"0.0.9"` in the workspace `Cargo.toml` **without committing it**; open Settings › "Verificar agora". Check: the notice fades in above "Novo projeto"; the popover opens on click; "Copiar comando" turns into "Copiado ✓" for 2 s and the clipboard holds the install command (paste in a terminal, do not run it); "Ver release" opens the GitHub page; "Não avisar desta versão" hides the notice and it stays hidden after a restart. Revert the version change (`git checkout Cargo.toml Cargo.lock`).

If the hidden `TextInput` does not copy (clipboard empty), report it instead of working around it — the fallback decision (e.g. the `arboard` crate) belongs to the user.

- [ ] **Step 6: Commit**

```bash
git add crates/app-slint
git commit -F - <<'EOF'
feat(app): show a quiet new-version notice in the sidebar

The popover shows the changelog excerpt, copies the installer command
for the current OS and opens the release page. A version can be
dismissed until a greater one ships.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```

---

### Task 11: Docs and final verification

**Files:**
- Modify: `README.md`, `docs/manual-checklist.md`

- [ ] **Step 1: README privacy note**

In `README.md`, after the paragraph that ends the "Install" section's "From source" block, add:

```markdown
### Privacy

ray-task never sends your tasks anywhere. Once a day it asks GitHub's public API for the latest
release version (`api.github.com/repos/carvalhosauro/ray-task/releases/latest`); nothing else is
sent. Turn it off in **Configurações › Atualizações › Verificar automaticamente**.
```

- [ ] **Step 2: Manual checklist**

In `docs/manual-checklist.md`, under `## Comportamento`, add:

```markdown
- [ ] Configurações (`Ctrl+,` e botão na barra lateral): Sistema/Claro/Escuro trocam na hora e persistem após reiniciar
- [ ] Tema "Sistema" segue a troca de tema do sistema com o app aberto
- [ ] `F1` e `?` abrem a lista de atalhos; `Esc` e clique fora fecham; `?` num campo de texto só digita
- [ ] Na tela de configurações, `Delete`/`Ctrl+Enter` não mexem na tarefa selecionada
- [ ] Aviso de versão nova (rodar com versão menor no Cargo.toml): aparece discreto; popover copia o comando, abre o release e "Não avisar desta versão" persiste
- [ ] Sem rede: nenhum aviso de erro aparece sozinho; "Verificar agora" mostra "Não foi possível verificar"
```

- [ ] **Step 3: Full verification**

Run, one at a time:

```bash
cargo fmt --check
CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=2 cargo test --workspace
```

Expected: no formatting diff, no clippy warnings, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/manual-checklist.md
git commit -F - <<'EOF'
docs: describe the update check and add manual checks for settings

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Xgb3jTNVPUDbx3N7xDFUjr
EOF
```
