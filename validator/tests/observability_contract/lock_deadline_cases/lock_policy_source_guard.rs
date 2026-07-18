use super::*;

#[test]
pub(crate) fn product_sources_forbid_blocking_identity_locks_and_deadline_resets() {
    let identity = include_str!("../../../src/observability/identity.rs");
    let locking = include_str!("../../../src/observability/locking.rs");
    let append = include_str!("../../../src/observability/store/store_limits.rs");
    let query = include_str!("../../../src/observability/store/event_query.rs");
    let lifecycle = include_str!("../../../src/observability/lifecycle.rs");

    for (name, source) in [
        ("identity", identity),
        ("locking", locking),
        ("append", append),
        ("query", query),
        ("lifecycle", lifecycle),
    ] {
        assert!(
            !source.contains(".lock()") && !source.contains("Mutex::lock"),
            "{name} restored an unbounded blocking mutex acquisition"
        );
    }
    assert_eq!(
        locking.matches("checked_add(STORE_LOCK_TIMEOUT)").count(),
        1,
        "the store deadline must be established in exactly one constructor"
    );
    assert!(
        !identity.contains("for_store_operation"),
        "identity acquisition must consume its caller's deadline"
    );
    assert!(
        !locking.contains("lock_file_with_deadline(file, kind, &LockDeadline"),
        "file acquisition must not create a replacement deadline"
    );
    assert_order(
        append,
        "open_for_append(&deadline)",
        "lock_exclusive(&file, &deadline)",
        "append lock order",
    );
    assert_order(
        query,
        "self.identity.expected(&deadline)",
        "lock_shared(&file, &deadline)",
        "read lock order",
    );
    assert_eq!(
        lifecycle
            .matches("LockDeadline::for_store_operation()")
            .count(),
        2,
        "clear and recovery each require one operation deadline"
    );
    assert_eq!(
        lifecycle
            .matches("lock_exclusive(&file, &deadline)")
            .count(),
        2,
        "clear and recovery must carry their original deadline to the file lock"
    );
}

pub(crate) fn assert_lock_timeout<T>(operation: impl FnOnce() -> Result<T, String>) {
    let started = Instant::now();
    let result = operation();
    let elapsed = started.elapsed();
    assert_timeout_result(result, elapsed);
}

pub(crate) fn assert_timeout_result<T>(result: Result<T, String>, elapsed: Duration) {
    let error = result.err().expect("contended operation must fail");
    assert_eq!(error, "observe-store-lock-timeout");
    assert!(EventStore::is_lock_timeout_error(&error));
    let timeout = lock_timeout();
    assert!(
        elapsed >= timeout.saturating_sub(Duration::from_millis(50)),
        "timeout returned too early: {elapsed:?}"
    );
    assert!(
        elapsed <= timeout + Duration::from_millis(750),
        "timeout was not bounded: {elapsed:?}"
    );
}

pub(crate) fn assert_order(source: &str, first: &str, second: &str, label: &str) {
    let first = source
        .find(first)
        .unwrap_or_else(|| panic!("{label}: missing {first}"));
    let second = source
        .find(second)
        .unwrap_or_else(|| panic!("{label}: missing {second}"));
    assert!(first < second, "{label}: lock acquisition was reordered");
}

pub(crate) fn lock_timeout() -> Duration {
    Duration::from_millis(EventStore::supported_lock_timeout_millis())
}

pub(crate) fn open_store_file(dir: &TestDir) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.store_path())
        .unwrap()
}

pub(crate) fn locked_exclusive(dir: &TestDir) -> File {
    let file = open_store_file(dir);
    file.lock().unwrap();
    file
}
