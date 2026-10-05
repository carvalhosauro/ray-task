use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use ray_core::{db, Due, FixedClock, Snapshot, Store, View};

#[test]
#[ignore = "medição: cargo test -p ray-core --release --test perf -- --ignored --nocapture"]
fn budgets_with_5000_tasks() {
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let projects: Vec<_> = (0..20).map(|i| store.create_project(&format!("Projeto {i}")).unwrap()).collect();
    let today = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
    for i in 0..5000usize {
        let id = store.create_task(View::Project(projects[i % 20]), &format!("Tarefa {i}")).unwrap();
        let offset = (i % 30) as i64 - 10;
        store.set_due(id, Some(Due { date: today + chrono::Duration::days(offset), time: None })).unwrap();
        if i % 7 == 0 {
            store.toggle_complete(id).unwrap();
        }
    }
    let ops = store.take_ops();

    let keep = HashSet::new();
    for view in [View::Today, View::Upcoming, View::Inbox, View::Project(projects[0])] {
        let start = Instant::now();
        for _ in 0..100 {
            std::hint::black_box(store.view(view, &keep));
        }
        let per_call = start.elapsed() / 100;
        println!("{view:?}: {per_call:?}");
        assert!(per_call < Duration::from_millis(2), "{view:?} levou {per_call:?}");
    }
    let start = Instant::now();
    for _ in 0..100 {
        std::hint::black_box(store.counts());
    }
    let per_call = start.elapsed() / 100;
    println!("counts: {per_call:?}");
    assert!(per_call < Duration::from_millis(2));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.db");
    {
        let mut conn = db::open(&path).unwrap();
        db::apply_all(&mut conn, &ops).unwrap();
    }
    let start = Instant::now();
    let conn = db::open(&path).unwrap();
    let snapshot = db::load(&conn).unwrap();
    let reloaded = Store::new(snapshot, Box::new(FixedClock::at("2026-10-05 13:35")));
    let elapsed = start.elapsed();
    println!("abrir + carregar: {elapsed:?}");
    assert_eq!(reloaded.counts().per_project.len(), 20);
    assert!(elapsed < Duration::from_millis(150), "abrir + carregar levou {elapsed:?}");
}
