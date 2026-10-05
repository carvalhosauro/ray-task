use chrono::NaiveDate;
use ray_core::{DomainError, Due, FixedClock, Snapshot, Store, View, WriteOp};

fn store() -> Store {
    Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")))
}

fn date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

#[test]
fn new_task_context_decides_project_and_due() {
    let mut s = store();
    let p = s.create_project("ray-task").unwrap();
    let today = s.create_task(View::Today, "a").unwrap();
    let upcoming = s.create_task(View::Upcoming, "b").unwrap();
    let inbox = s.create_task(View::Inbox, "c").unwrap();
    let in_project = s.create_task(View::Project(p), "d").unwrap();

    assert_eq!(s.task(today).unwrap().due, Some(Due { date: date("2026-10-05"), time: None }));
    assert_eq!(s.task(today).unwrap().project_id, None);
    assert_eq!(s.task(upcoming).unwrap().due, Some(Due { date: date("2026-10-06"), time: None }));
    assert_eq!(s.task(inbox).unwrap().due, None);
    assert_eq!(s.task(in_project).unwrap().project_id, Some(p));
    assert_eq!(s.task(in_project).unwrap().due, None);
}

#[test]
fn titles_are_normalized_and_blank_is_rejected() {
    let mut s = store();
    let id = s.create_task(View::Inbox, "  Pagar\nboleto ").unwrap();
    assert_eq!(s.task(id).unwrap().title, "Pagar boleto");
    assert_eq!(s.create_task(View::Inbox, " \n "), Err(DomainError::EmptyTitle));
    assert_eq!(s.update_text(id, "   ", "x"), Err(DomainError::EmptyTitle));
    assert_eq!(s.task(id).unwrap().title, "Pagar boleto");
}

#[test]
fn task_in_missing_project_is_rejected() {
    let mut s = store();
    assert_eq!(s.create_task(View::Project(42), "x"), Err(DomainError::ProjectNotFound(42)));
}

#[test]
fn toggle_complete_sets_and_clears_completed_at() {
    let mut s = store();
    let id = s.create_task(View::Inbox, "x").unwrap();
    assert_eq!(s.toggle_complete(id), Ok(true));
    assert!(s.task(id).unwrap().is_done());
    assert_eq!(s.toggle_complete(id), Ok(false));
    assert!(!s.task(id).unwrap().is_done());
}

#[test]
fn tags_belong_to_their_project() {
    let mut s = store();
    let a = s.create_project("A").unwrap();
    let b = s.create_project("B").unwrap();
    let task = s.create_task(View::Project(a), "x").unwrap();
    let tag_b = s.ensure_tag(b, "dev").unwrap();
    assert_eq!(s.add_tag(task, tag_b), Err(DomainError::TagProjectMismatch));

    let inbox = s.create_task(View::Inbox, "y").unwrap();
    let tag_a = s.ensure_tag(a, "dev").unwrap();
    assert_eq!(s.add_tag(inbox, tag_a), Err(DomainError::TagProjectMismatch));
}

#[test]
fn moving_a_task_clears_its_tags() {
    let mut s = store();
    let a = s.create_project("A").unwrap();
    let b = s.create_project("B").unwrap();
    let task = s.create_task(View::Project(a), "x").unwrap();
    let tag = s.ensure_tag(a, "dev").unwrap();
    s.add_tag(task, tag).unwrap();
    s.move_task(task, Some(b)).unwrap();
    assert_eq!(s.task(task).unwrap().project_id, Some(b));
    assert!(s.task(task).unwrap().tags.is_empty());
    s.move_task(task, None).unwrap();
    assert_eq!(s.task(task).unwrap().project_id, None);
}

#[test]
fn ensure_tag_reuses_names_ignoring_case_and_accents_case() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let first = s.ensure_tag(p, "Ação").unwrap();
    assert_eq!(s.ensure_tag(p, "#AÇÃO").unwrap(), first);
    assert_eq!(s.ensure_tag(p, " ação ").unwrap(), first);
    assert_eq!(s.project_tags(p).len(), 1);
    assert_eq!(s.ensure_tag(p, "#"), Err(DomainError::EmptyName));
}

#[test]
fn rename_tag_rejects_duplicates_in_same_project() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let dev = s.ensure_tag(p, "dev").unwrap();
    let ops = s.ensure_tag(p, "ops").unwrap();
    assert_eq!(s.rename_tag(ops, "DEV"), Err(DomainError::DuplicateTag));
    s.rename_tag(dev, "Backend").unwrap();
    assert_eq!(s.tag(dev).unwrap().name, "Backend");
}

#[test]
fn deleting_a_project_removes_its_tasks_and_tags() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let t1 = s.create_task(View::Project(p), "x").unwrap();
    s.create_task(View::Project(p), "y").unwrap();
    s.toggle_complete(t1).unwrap();
    let keep = s.create_task(View::Inbox, "z").unwrap();
    s.ensure_tag(p, "dev").unwrap();
    assert_eq!(s.task_count_in_project(p), 2);
    assert_eq!(s.delete_project(p), Ok(2));
    assert!(s.project(p).is_none());
    assert!(s.project_tags(p).is_empty());
    assert!(s.task(t1).is_none());
    assert!(s.task(keep).is_some());
}

#[test]
fn deleting_a_tag_removes_it_from_tasks() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let task = s.create_task(View::Project(p), "x").unwrap();
    let tag = s.ensure_tag(p, "dev").unwrap();
    s.add_tag(task, tag).unwrap();
    s.delete_tag(tag).unwrap();
    assert!(s.task(task).unwrap().tags.is_empty());
    assert!(s.tag(tag).is_none());
}

#[test]
fn mutations_emit_write_ops_in_order() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let t = s.create_task(View::Project(p), "x").unwrap();
    s.delete_task(t).unwrap();
    let ops = s.take_ops();
    assert!(matches!(ops[0], WriteOp::UpsertProject(ref pr) if pr.id == p));
    assert!(matches!(ops[1], WriteOp::UpsertTask(ref tk) if tk.id == t));
    assert_eq!(ops[2], WriteOp::DeleteTask(t));
    assert!(s.take_ops().is_empty());
}

#[test]
fn ids_continue_after_loaded_snapshot() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let t = s.create_task(View::Inbox, "x").unwrap();
    let ops = s.take_ops();
    let mut conn = ray_core::db::open_in_memory().unwrap();
    ray_core::db::apply_all(&mut conn, &ops).unwrap();
    let mut reloaded = Store::new(ray_core::db::load(&conn).unwrap(), Box::new(FixedClock::at("2026-10-05 13:35")));
    assert_eq!(reloaded.create_project("B").unwrap(), p + 1);
    assert_eq!(reloaded.create_task(View::Inbox, "y").unwrap(), t + 1);
}

#[test]
fn project_colors_cycle_through_palette() {
    let mut s = store();
    let a = s.create_project("A").unwrap();
    let b = s.create_project("B").unwrap();
    assert_eq!(s.project(a).unwrap().color, "#0A84FF");
    assert_eq!(s.project(b).unwrap().color, "#FF9F0A");
    s.set_project_color(a, "#30D158").unwrap();
    assert_eq!(s.project(a).unwrap().color, "#30D158");
    s.rename_project(a, " Casa ").unwrap();
    assert_eq!(s.project(a).unwrap().name, "Casa");
    assert_eq!(s.rename_project(a, " "), Err(DomainError::EmptyName));
}
