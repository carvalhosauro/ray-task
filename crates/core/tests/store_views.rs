use std::collections::HashSet;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use ray_core::{due_status, matches_query, Due, DueStatus, FixedClock, Snapshot, Store, TaskId, View};

fn setup() -> (Store, FixedClock) {
    let clock = FixedClock::at("2026-10-05 13:35");
    (Store::new(Snapshot::default(), Box::new(clock.clone())), clock)
}

fn due(date: &str, time: Option<&str>) -> Option<Due> {
    Some(Due {
        date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
        time: time.map(|t| NaiveTime::parse_from_str(t, "%H:%M").unwrap()),
    })
}

fn add(s: &mut Store, title: &str, d: Option<Due>) -> TaskId {
    let id = s.create_task(View::Inbox, title).unwrap();
    s.set_due(id, d).unwrap();
    id
}

fn ids(tasks: Vec<&ray_core::Task>) -> Vec<TaskId> {
    tasks.into_iter().map(|t| t.id).collect()
}

#[test]
fn today_lists_overdue_then_timed_then_untimed() {
    let (mut s, _) = setup();
    let untimed = add(&mut s, "sem hora", due("2026-10-05", None));
    let at_14 = add(&mut s, "14h", due("2026-10-05", Some("14:00")));
    let yesterday = add(&mut s, "ontem", due("2026-10-04", None));
    let at_13 = add(&mut s, "13h", due("2026-10-05", Some("13:00")));
    add(&mut s, "amanhã", due("2026-10-06", None));
    add(&mut s, "sem data", None);
    assert_eq!(ids(s.view(View::Today, &HashSet::new())), vec![yesterday, at_13, at_14, untimed]);
}

#[test]
fn upcoming_lists_future_by_date_then_time() {
    let (mut s, _) = setup();
    let fri = add(&mut s, "sex", due("2026-10-09", None));
    let tue_late = add(&mut s, "ter 18", due("2026-10-06", Some("18:00")));
    let tue_early = add(&mut s, "ter 08", due("2026-10-06", Some("08:00")));
    let tue_untimed = add(&mut s, "ter", due("2026-10-06", None));
    add(&mut s, "hoje", due("2026-10-05", None));
    assert_eq!(ids(s.view(View::Upcoming, &HashSet::new())), vec![tue_early, tue_late, tue_untimed, fri]);
}

#[test]
fn completed_tasks_are_hidden_unless_kept() {
    let (mut s, _) = setup();
    let p = s.create_project("A").unwrap();
    let a = s.create_task(View::Project(p), "a").unwrap();
    let b = s.create_task(View::Project(p), "b").unwrap();
    s.toggle_complete(a).unwrap();
    assert_eq!(ids(s.view(View::Project(p), &HashSet::new())), vec![b]);
    assert_eq!(ids(s.view(View::Project(p), &HashSet::from([a]))), vec![a, b]);
    assert_eq!(ids(s.completed_in_project(p)), vec![a]);
}

#[test]
fn inbox_only_has_tasks_without_project() {
    let (mut s, _) = setup();
    let p = s.create_project("A").unwrap();
    s.create_task(View::Project(p), "a").unwrap();
    let inbox = s.create_task(View::Inbox, "b").unwrap();
    assert_eq!(ids(s.view(View::Inbox, &HashSet::new())), vec![inbox]);
}

#[test]
fn counts_flag_overdue_including_past_time_today() {
    let (mut s, _) = setup();
    let later = add(&mut s, "14h", due("2026-10-05", Some("14:00")));
    assert!(!s.counts().overdue);
    let past = add(&mut s, "13h", due("2026-10-05", Some("13:00")));
    let counts = s.counts();
    assert!(counts.overdue);
    assert_eq!(counts.today, 2);
    assert_eq!(counts.inbox, 2);
    s.toggle_complete(past).unwrap();
    assert!(!s.counts().overdue);
    assert!(!s.is_late(s.task(later).unwrap()));
}

#[test]
fn counts_per_project_ignore_completed() {
    let (mut s, _) = setup();
    let p = s.create_project("A").unwrap();
    let a = s.create_task(View::Project(p), "a").unwrap();
    s.create_task(View::Project(p), "b").unwrap();
    s.toggle_complete(a).unwrap();
    assert_eq!(s.counts().per_project.get(&p), Some(&1));
    let empty = s.create_project("B").unwrap();
    assert_eq!(s.counts().per_project.get(&empty), Some(&0));
}

#[test]
fn day_rollover_moves_tomorrow_into_today() {
    let (mut s, clock) = setup();
    let tomorrow = add(&mut s, "amanhã", due("2026-10-06", None));
    assert!(ids(s.view(View::Today, &HashSet::new())).is_empty());
    clock.set("2026-10-06 00:01");
    assert_eq!(ids(s.view(View::Today, &HashSet::new())), vec![tomorrow]);
    assert!(ids(s.view(View::Upcoming, &HashSet::new())).is_empty());
}

#[test]
fn due_status_reports_relative_minutes() {
    let now = NaiveDateTime::parse_from_str("2026-10-05 13:35", "%Y-%m-%d %H:%M").unwrap();
    assert_eq!(due_status(None, now), DueStatus::NoDate);
    assert_eq!(due_status(due("2026-10-03", None), now), DueStatus::Overdue { days: 2 });
    assert_eq!(due_status(due("2026-10-05", None), now), DueStatus::Today);
    assert_eq!(due_status(due("2026-10-05", Some("14:00")), now), DueStatus::TodayAt { minutes_until: 25 });
    assert_eq!(due_status(due("2026-10-05", Some("13:00")), now), DueStatus::TodayAt { minutes_until: -35 });
    assert_eq!(due_status(due("2026-10-06", Some("09:00")), now), DueStatus::Future);
}

#[test]
fn query_matches_title_and_notes_ignoring_case() {
    let (mut s, _) = setup();
    let id = s.create_task(View::Inbox, "Revisar AÇÃO").unwrap();
    s.update_text(id, "Revisar AÇÃO", "ligar pro João").unwrap();
    let task = s.task(id).unwrap();
    assert!(matches_query(task, "ação"));
    assert!(matches_query(task, "JOÃO"));
    assert!(matches_query(task, "  "));
    assert!(!matches_query(task, "boleto"));
}
