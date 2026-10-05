use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ray_core::writer::{Sink, Writer, WriterEvent};
use ray_core::WriteOp;

#[derive(Clone, Default)]
struct Disk {
    applied: Arc<Mutex<Vec<WriteOp>>>,
    fail_next: Arc<AtomicUsize>,
    always_fail: Arc<AtomicBool>,
}

impl Sink for Disk {
    fn apply(&mut self, op: &WriteOp) -> Result<(), String> {
        if self.always_fail.load(SeqCst) {
            return Err("disco cheio".into());
        }
        if self.fail_next.load(SeqCst) > 0 {
            self.fail_next.fetch_sub(1, SeqCst);
            return Err("disco cheio".into());
        }
        self.applied.lock().unwrap().push(op.clone());
        Ok(())
    }
}

fn op(id: i64) -> WriteOp {
    WriteOp::DeleteTask(id)
}

fn wait_until(cond: impl Fn() -> bool) {
    let start = Instant::now();
    while !cond() {
        assert!(start.elapsed() < Duration::from_secs(2), "condição não aconteceu a tempo");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn applies_ops_in_order_and_drains_on_shutdown() {
    let disk = Disk::default();
    let writer = Writer::spawn(disk.clone(), |_| {});
    let handle = writer.handle();
    handle.send(vec![op(1), op(2)]);
    handle.send(vec![]);
    handle.send(vec![op(3)]);
    assert_eq!(writer.shutdown(Duration::from_secs(2)), Some(0));
    assert_eq!(*disk.applied.lock().unwrap(), vec![op(1), op(2), op(3)]);
}

#[test]
fn failure_keeps_op_queued_until_retry() {
    let disk = Disk::default();
    disk.fail_next.store(1, SeqCst);
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let writer = Writer::spawn(disk.clone(), move |e| sink.lock().unwrap().push(e));
    let handle = writer.handle();
    handle.send(vec![op(1), op(2)]);
    wait_until(|| !events.lock().unwrap().is_empty());
    assert_eq!(events.lock().unwrap()[0], WriterEvent::Failed { error: "disco cheio".into(), pending: 2 });
    assert!(disk.applied.lock().unwrap().is_empty());

    handle.retry();
    assert_eq!(writer.shutdown(Duration::from_secs(2)), Some(0));
    assert_eq!(*disk.applied.lock().unwrap(), vec![op(1), op(2)]);
    assert_eq!(events.lock().unwrap().last(), Some(&WriterEvent::Recovered));
}

#[test]
fn shutdown_does_not_hang_when_disk_keeps_failing() {
    let disk = Disk::default();
    disk.always_fail.store(true, SeqCst);
    let writer = Writer::spawn(disk, |_| {});
    writer.handle().send(vec![op(1)]);
    let start = Instant::now();
    assert_eq!(writer.shutdown(Duration::from_secs(2)), Some(1));
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn sqlite_sink_writes_to_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("w.db");
    let conn = ray_core::db::open(&path).unwrap();
    let mut store = ray_core::Store::new(ray_core::Snapshot::default(), Box::new(ray_core::FixedClock::at("2026-10-05 13:35")));
    store.create_task(ray_core::View::Inbox, "gravar").unwrap();
    let writer = Writer::spawn(ray_core::writer::SqliteSink(conn), |_| {});
    writer.handle().send(store.take_ops());
    assert_eq!(writer.shutdown(Duration::from_secs(2)), Some(0));
    let reopened = ray_core::db::open(&path).unwrap();
    assert_eq!(ray_core::db::load(&reopened).unwrap().tasks[0].title, "gravar");
}
