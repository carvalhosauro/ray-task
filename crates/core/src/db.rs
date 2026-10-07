use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, NaiveDate, NaiveTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, Transaction};

use crate::model::{Due, Project, Snapshot, Tag, TagId, Task, TaskId};
use crate::ops::WriteOp;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("erro do SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro de arquivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("dado inválido no banco: {0}")]
    Corrupt(String),
    #[error("o banco foi criado por uma versão mais nova do ray-task (schema {found}, suportado {supported})")]
    TooNew { found: i64, supported: i64 },
}

const MIGRATIONS: &[&str] = &[include_str!("migrations/001_init.sql"), include_str!("migrations/002_settings.sql")];
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// Abre (criando pastas e arquivo se preciso), faz backup se houver migration pendente e migra.
pub fn open(path: &Path) -> Result<Connection, DbError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    let version = user_version(&conn)?;
    if version > 0 && version < SCHEMA_VERSION {
        backup(&conn, &path.with_extension("db.bak"))?;
    }
    migrate(&mut conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection, DbError> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<(), DbError> {
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    Ok(())
}

fn user_version(conn: &Connection) -> Result<i64, DbError> {
    Ok(conn.pragma_query_value(None, "user_version", |r| r.get(0))?)
}

pub fn migrate(conn: &mut Connection) -> Result<(), DbError> {
    let current = user_version(conn)?;
    if current > SCHEMA_VERSION {
        return Err(DbError::TooNew { found: current, supported: SCHEMA_VERSION });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

/// Cópia consistente (inclui o que ainda está no WAL). Sobrescreve `dest`.
pub fn backup(conn: &Connection, dest: &Path) -> Result<(), DbError> {
    match std::fs::remove_file(dest) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy().to_string()])?;
    Ok(())
}

pub fn load(conn: &Connection) -> Result<Snapshot, DbError> {
    let mut snap = Snapshot::default();

    let mut stmt = conn.prepare("SELECT id, name, color, sort_order, created_at FROM projects ORDER BY sort_order, id")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?, r.get::<_, String>(4)?))
    })?;
    for row in rows {
        let (id, name, color, sort_order, created_at) = row?;
        snap.projects.push(Project { id, name, color, sort_order, created_at: parse_utc(&created_at)? });
    }

    let mut stmt = conn.prepare("SELECT id, project_id, name, color FROM tags ORDER BY id")?;
    let rows = stmt.query_map([], |r| Ok(Tag { id: r.get(0)?, project_id: r.get(1)?, name: r.get(2)?, color: r.get(3)? }))?;
    for row in rows {
        snap.tags.push(row?);
    }

    let mut tags_by_task: HashMap<TaskId, Vec<TagId>> = HashMap::new();
    let mut stmt = conn.prepare("SELECT task_id, tag_id FROM task_tags ORDER BY tag_id")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    for row in rows {
        let (task_id, tag_id) = row?;
        tags_by_task.entry(task_id).or_default().push(tag_id);
    }

    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, notes, due_date, due_time, completed_at, sort_order, created_at, updated_at
         FROM tasks ORDER BY sort_order, id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, Option<i64>>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, i64>(7)?,
            r.get::<_, String>(8)?,
            r.get::<_, String>(9)?,
        ))
    })?;
    for row in rows {
        let (id, project_id, title, notes, due_date, due_time, completed_at, sort_order, created_at, updated_at) = row?;
        let due = match due_date {
            Some(date) => Some(Due { date: parse_date(&date)?, time: due_time.as_deref().map(parse_time).transpose()? }),
            None => None,
        };
        snap.tasks.push(Task {
            id,
            project_id,
            title,
            notes,
            due,
            completed_at: completed_at.as_deref().map(parse_utc).transpose()?,
            sort_order,
            created_at: parse_utc(&created_at)?,
            updated_at: parse_utc(&updated_at)?,
            tags: tags_by_task.remove(&id).unwrap_or_default(),
        });
    }

    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (key, value) = row?;
        if let Err(error) = snap.settings.apply(&key, &value) {
            tracing::warn!(%key, %value, %error, "configuração inválida, usando o padrão");
        }
    }
    Ok(snap)
}

/// Aplica uma operação na sua própria transação.
pub fn apply(conn: &mut Connection, op: &WriteOp) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    apply_in(&tx, op)?;
    tx.commit()?;
    Ok(())
}

/// Aplica várias operações numa transação só (usado em testes e cargas grandes).
pub fn apply_all(conn: &mut Connection, ops: &[WriteOp]) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    for op in ops {
        apply_in(&tx, op)?;
    }
    tx.commit()?;
    Ok(())
}

fn apply_in(tx: &Transaction, op: &WriteOp) -> Result<(), DbError> {
    match op {
        WriteOp::UpsertProject(p) => {
            tx.execute(
                "INSERT INTO projects (id, name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, color = excluded.color, sort_order = excluded.sort_order",
                params![p.id, p.name, p.color, p.sort_order, fmt_utc(&p.created_at)],
            )?;
        }
        WriteOp::DeleteProject(id) => {
            tx.execute("DELETE FROM projects WHERE id = ?1", [id])?;
        }
        WriteOp::UpsertTag(t) => {
            tx.execute(
                "INSERT INTO tags (id, project_id, name, color) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, color = excluded.color",
                params![t.id, t.project_id, t.name, t.color],
            )?;
        }
        WriteOp::DeleteTag(id) => {
            tx.execute("DELETE FROM tags WHERE id = ?1", [id])?;
        }
        WriteOp::UpsertTask(t) => {
            tx.execute(
                "INSERT INTO tasks (id, project_id, title, notes, due_date, due_time, completed_at, sort_order, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                   project_id = excluded.project_id, title = excluded.title, notes = excluded.notes,
                   due_date = excluded.due_date, due_time = excluded.due_time, completed_at = excluded.completed_at,
                   sort_order = excluded.sort_order, updated_at = excluded.updated_at",
                params![
                    t.id,
                    t.project_id,
                    t.title,
                    t.notes,
                    t.due.map(|d| d.date.format("%Y-%m-%d").to_string()),
                    t.due.and_then(|d| d.time).map(|tm| tm.format("%H:%M").to_string()),
                    t.completed_at.as_ref().map(fmt_utc),
                    t.sort_order,
                    fmt_utc(&t.created_at),
                    fmt_utc(&t.updated_at),
                ],
            )?;
            tx.execute("DELETE FROM task_tags WHERE task_id = ?1", [t.id])?;
            for tag in &t.tags {
                tx.execute("INSERT INTO task_tags (task_id, tag_id) VALUES (?1, ?2)", [t.id, *tag])?;
            }
        }
        WriteOp::DeleteTask(id) => {
            tx.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        }
        WriteOp::SetSetting { key, value } => {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
    }
    Ok(())
}

fn fmt_utc(d: &DateTime<Utc>) -> String {
    d.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_utc(s: &str) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(s).map(|d| d.with_timezone(&Utc)).map_err(|e| DbError::Corrupt(format!("data/hora '{s}': {e}")))
}

fn parse_date(s: &str) -> Result<NaiveDate, DbError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| DbError::Corrupt(format!("data '{s}': {e}")))
}

fn parse_time(s: &str) -> Result<NaiveTime, DbError> {
    NaiveTime::parse_from_str(s, "%H:%M").map_err(|e| DbError::Corrupt(format!("hora '{s}': {e}")))
}
