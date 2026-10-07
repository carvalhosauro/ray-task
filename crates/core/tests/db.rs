use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use ray_core::db::{self, DbError};
use ray_core::{Due, FixedClock, Project, Settings, Snapshot, Store, Tag, Task, ThemeMode, WriteOp};

fn ts(s: &str) -> DateTime<Utc> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap().and_utc()
}

fn project(id: i64) -> Project {
    Project { id, name: format!("P{id}"), color: "#0A84FF".into(), sort_order: id, created_at: ts("2026-10-01 09:00") }
}

fn tag(id: i64, project_id: i64, name: &str) -> Tag {
    Tag { id, project_id, name: name.into(), color: "#8E8E93".into() }
}

fn task(id: i64, project_id: Option<i64>) -> Task {
    Task {
        id,
        project_id,
        title: format!("T{id}"),
        notes: String::new(),
        due: None,
        completed_at: None,
        sort_order: id,
        created_at: ts("2026-10-01 09:00"),
        updated_at: ts("2026-10-01 09:00"),
        tags: vec![],
    }
}

#[test]
fn fresh_database_is_migrated_to_current_version() {
    let conn = db::open_in_memory().unwrap();
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
    assert_eq!(version, db::SCHEMA_VERSION);
}

#[test]
fn roundtrip_preserves_every_field() {
    let mut conn = db::open_in_memory().unwrap();
    let mut full = task(1, Some(1));
    full.notes = "linha 1\nlinha 2".into();
    full.due = Some(Due { date: NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(), time: Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap()) });
    full.completed_at = Some(ts("2026-10-05 15:00"));
    full.tags = vec![10];
    let inbox = task(2, None);
    db::apply_all(
        &mut conn,
        &[
            WriteOp::UpsertProject(project(1)),
            WriteOp::UpsertTag(tag(10, 1, "dev")),
            WriteOp::UpsertTask(full.clone()),
            WriteOp::UpsertTask(inbox.clone()),
        ],
    )
    .unwrap();

    let snap = db::load(&conn).unwrap();
    assert_eq!(snap.projects, vec![project(1)]);
    assert_eq!(snap.tags, vec![tag(10, 1, "dev")]);
    assert_eq!(snap.tasks.iter().find(|t| t.id == 1), Some(&full));
    assert_eq!(snap.tasks.iter().find(|t| t.id == 2), Some(&inbox));
}

#[test]
fn upsert_updates_existing_rows() {
    let mut conn = db::open_in_memory().unwrap();
    db::apply(&mut conn, &WriteOp::UpsertTask(task(1, None))).unwrap();
    let mut renamed = task(1, None);
    renamed.title = "Novo título".into();
    db::apply(&mut conn, &WriteOp::UpsertTask(renamed.clone())).unwrap();
    assert_eq!(db::load(&conn).unwrap().tasks, vec![renamed]);
}

#[test]
fn trigger_rejects_tag_from_another_project() {
    let mut conn = db::open_in_memory().unwrap();
    db::apply_all(
        &mut conn,
        &[WriteOp::UpsertProject(project(1)), WriteOp::UpsertProject(project(2)), WriteOp::UpsertTag(tag(10, 2, "dev"))],
    )
    .unwrap();
    let mut wrong = task(1, Some(1));
    wrong.tags = vec![10];
    assert!(db::apply(&mut conn, &WriteOp::UpsertTask(wrong)).is_err());
    assert!(db::load(&conn).unwrap().tasks.is_empty(), "a transação inteira deve ser desfeita");
}

#[test]
fn trigger_rejects_tags_on_inbox_task() {
    let mut conn = db::open_in_memory().unwrap();
    db::apply_all(&mut conn, &[WriteOp::UpsertProject(project(1)), WriteOp::UpsertTag(tag(10, 1, "dev"))]).unwrap();
    let mut inbox = task(1, None);
    inbox.tags = vec![10];
    assert!(db::apply(&mut conn, &WriteOp::UpsertTask(inbox)).is_err());
}

#[test]
fn blank_title_is_rejected_by_schema() {
    let mut conn = db::open_in_memory().unwrap();
    let mut blank = task(1, None);
    blank.title = "   ".into();
    assert!(db::apply(&mut conn, &WriteOp::UpsertTask(blank)).is_err());
}

#[test]
fn deleting_project_cascades_to_tasks_and_tags() {
    let mut conn = db::open_in_memory().unwrap();
    let mut tagged = task(1, Some(1));
    tagged.tags = vec![10];
    db::apply_all(
        &mut conn,
        &[
            WriteOp::UpsertProject(project(1)),
            WriteOp::UpsertTag(tag(10, 1, "dev")),
            WriteOp::UpsertTask(tagged),
            WriteOp::UpsertTask(task(2, None)),
        ],
    )
    .unwrap();
    db::apply(&mut conn, &WriteOp::DeleteProject(1)).unwrap();
    let snap = db::load(&conn).unwrap();
    assert!(snap.projects.is_empty());
    assert!(snap.tags.is_empty());
    assert_eq!(snap.tasks.iter().map(|t| t.id).collect::<Vec<_>>(), vec![2]);
}

#[test]
fn file_database_reopens_with_data_in_wal_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("ray-task.db");
    {
        let mut conn = db::open(&path).unwrap();
        db::apply(&mut conn, &WriteOp::UpsertProject(project(1))).unwrap();
    }
    let conn = db::open(&path).unwrap();
    let mode: String = conn.pragma_query_value(None, "journal_mode", |r| r.get(0)).unwrap();
    assert_eq!(mode, "wal");
    assert_eq!(db::load(&conn).unwrap().projects, vec![project(1)]);
}

#[test]
fn newer_schema_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ray-task.db");
    {
        let conn = db::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
    }
    assert!(matches!(db::open(&path), Err(DbError::TooNew { found: 99, .. })));
}

#[test]
fn backup_writes_a_readable_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ray-task.db");
    let bak = dir.path().join("ray-task.db.bak");
    let mut conn = db::open(&path).unwrap();
    db::apply(&mut conn, &WriteOp::UpsertProject(project(1))).unwrap();
    db::backup(&conn, &bak).unwrap();
    db::backup(&conn, &bak).unwrap(); // sobrescrever também funciona
    let copy = rusqlite::Connection::open(&bak).unwrap();
    let n: i64 = copy.query_row("SELECT count(*) FROM projects", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 1);
}

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
    conn.execute_batch("INSERT INTO settings (key, value) VALUES ('theme', 'purple'), ('update_check', 'maybe'), ('future_key', 'x');")
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
