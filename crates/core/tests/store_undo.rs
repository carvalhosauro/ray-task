use ray_core::{FixedClock, Snapshot, Store, View, WriteOp};

fn store() -> Store {
    Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")))
}

#[test]
fn undo_restores_deleted_task_with_tags() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let t = s.create_task(View::Project(p), "x").unwrap();
    let tag = s.ensure_tag(p, "dev").unwrap();
    s.add_tag(t, tag).unwrap();
    let before = s.task(t).unwrap().clone();
    s.delete_task(t).unwrap();
    s.take_ops();
    assert!(s.undo());
    let restored = s.task(t).unwrap();
    assert_eq!(restored.title, before.title);
    assert_eq!(restored.tags, vec![tag]);
    assert_eq!(restored.sort_order, before.sort_order);
    assert!(matches!(s.take_ops().as_slice(), [WriteOp::UpsertTask(task)] if task.id == t));
}

#[test]
fn undo_reverts_completion() {
    let mut s = store();
    let t = s.create_task(View::Inbox, "x").unwrap();
    s.toggle_complete(t).unwrap();
    assert!(s.undo());
    assert!(!s.task(t).unwrap().is_done());
}

#[test]
fn undo_move_restores_project_and_tags() {
    let mut s = store();
    let a = s.create_project("A").unwrap();
    let t = s.create_task(View::Project(a), "x").unwrap();
    let tag = s.ensure_tag(a, "dev").unwrap();
    s.add_tag(t, tag).unwrap();
    s.move_task(t, None).unwrap();
    assert!(s.undo());
    assert_eq!(s.task(t).unwrap().project_id, Some(a));
    assert_eq!(s.task(t).unwrap().tags, vec![tag]);
}

#[test]
fn undo_applies_most_recent_first() {
    let mut s = store();
    let a = s.create_task(View::Inbox, "a").unwrap();
    let b = s.create_task(View::Inbox, "b").unwrap();
    s.toggle_complete(a).unwrap();
    s.delete_task(b).unwrap();
    assert!(s.undo());
    assert!(s.task(b).is_some());
    assert!(s.task(a).unwrap().is_done());
    assert!(s.undo());
    assert!(!s.task(a).unwrap().is_done());
    assert!(!s.undo());
    assert!(!s.can_undo());
}

#[test]
fn undo_history_is_capped_at_20() {
    let mut s = store();
    let t = s.create_task(View::Inbox, "x").unwrap();
    for _ in 0..25 {
        s.toggle_complete(t).unwrap();
    }
    let mut undone = 0;
    while s.undo() {
        undone += 1;
    }
    assert_eq!(undone, 20);
}

#[test]
fn undo_skips_entries_whose_project_was_deleted() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let in_project = s.create_task(View::Project(p), "x").unwrap();
    let inbox = s.create_task(View::Inbox, "y").unwrap();
    s.toggle_complete(inbox).unwrap();
    s.delete_task(in_project).unwrap();
    s.delete_project(p).unwrap();
    assert!(s.undo());
    assert!(s.task(in_project).is_none(), "tarefa de projeto apagado não volta");
    assert!(!s.task(inbox).unwrap().is_done());
}

#[test]
fn undo_drops_tags_deleted_in_the_meantime() {
    let mut s = store();
    let p = s.create_project("A").unwrap();
    let t = s.create_task(View::Project(p), "x").unwrap();
    let tag = s.ensure_tag(p, "dev").unwrap();
    s.add_tag(t, tag).unwrap();
    s.delete_task(t).unwrap();
    s.delete_tag(tag).unwrap();
    assert!(s.undo());
    assert!(s.task(t).unwrap().tags.is_empty());
}
