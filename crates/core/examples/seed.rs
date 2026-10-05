//! Popula um banco com 5.000 tarefas (mesmo cenário do teste de perf).
//! Uso: cargo run -p ray-core --release --example seed -- <caminho.db>

use chrono::NaiveDate;
use ray_core::{db, Due, FixedClock, Snapshot, Store, View};

fn main() {
    let path = std::env::args().nth(1).expect("uso: seed <caminho.db>");
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
    let mut conn = db::open(std::path::Path::new(&path)).unwrap();
    db::apply_all(&mut conn, &ops).unwrap();
    println!("5000 tarefas gravadas em {path}");
}
