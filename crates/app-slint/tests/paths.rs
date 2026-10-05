use ray_task::paths::{acquire_lock, LockError};

#[test]
fn second_lock_on_same_file_fails_until_first_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("run").join("ray-task.lock");
    let first = acquire_lock(&path).unwrap();
    assert!(matches!(acquire_lock(&path), Err(LockError::AlreadyRunning)));
    drop(first);
    assert!(acquire_lock(&path).is_ok());
}
