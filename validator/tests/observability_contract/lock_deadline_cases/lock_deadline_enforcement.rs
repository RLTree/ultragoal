use super::*;

#[test]
pub(crate) fn exclusive_contention_has_one_deadline_for_query_append_clear_recovery_and_export_reads()
 {
    let dir = TestDir::new("exclusive-lock-deadline");
    let store = store(&dir);
    assert!(store.append(&event("seed", 1, 1, "pass")).unwrap());
    let before = tree_snapshot(dir.path());
    let holder = locked_exclusive(&dir);

    assert_lock_timeout(|| store.query(&query()));
    assert_lock_timeout(|| store.append(&event("blocked", 2, 2, "fail")));
    assert_lock_timeout(|| store.clear());
    assert_lock_timeout(|| store.recover_truncated_tail());

    let mut adapter = MockAdapter::new(AdapterMode::Ok);
    assert_lock_timeout(|| {
        store.export_explicit(ExplicitExportRequest {
            query: &query(),
            configured: true,
            consent_granted: true,
            timeout: Duration::from_secs(2),
            adapter: Some(&mut adapter),
        })
    });
    assert_eq!(
        adapter.export_calls, 0,
        "a contended read must not reach export"
    );
    assert_eq!(
        tree_snapshot(dir.path()),
        before,
        "contention must not write"
    );

    holder.unlock().unwrap();
    assert_eq!(store.query(&query()).unwrap().len(), 1);
}

#[test]
pub(crate) fn shared_holder_allows_reads_but_exclusive_append_times_out_without_writing() {
    let dir = TestDir::new("shared-lock-deadline");
    let store = store(&dir);
    assert!(store.append(&event("seed", 1, 1, "pass")).unwrap());
    let before = tree_snapshot(dir.path());
    let holder = open_store_file(&dir);
    holder.lock_shared().unwrap();

    assert_eq!(store.query(&query()).unwrap().len(), 1);
    let mut adapter = MockAdapter::new(AdapterMode::Ok);
    assert_eq!(
        store
            .export_explicit(ExplicitExportRequest {
                query: &query(),
                configured: true,
                consent_granted: true,
                timeout: Duration::from_secs(2),
                adapter: Some(&mut adapter),
            })
            .unwrap(),
        1
    );
    assert_eq!(adapter.export_calls, 1);
    assert_lock_timeout(|| store.append(&event("blocked", 2, 2, "fail")));
    assert_eq!(
        tree_snapshot(dir.path()),
        before,
        "blocked append wrote bytes"
    );

    holder.unlock().unwrap();
    assert!(store.append(&event("released", 2, 2, "pass")).unwrap());
}

#[test]
pub(crate) fn release_before_the_monotonic_deadline_succeeds_and_timeout_is_not_corruption() {
    let dir = TestDir::new("release-before-lock-deadline");
    let store = store(&dir);
    assert!(store.append(&event("seed", 1, 1, "pass")).unwrap());
    let holder = locked_exclusive(&dir);
    let timeout = lock_timeout();
    let release_after = timeout
        .checked_sub(Duration::from_millis(250))
        .expect("lock timeout leaves a release margin");
    let release = std::thread::spawn(move || {
        std::thread::sleep(release_after);
        holder.unlock().unwrap();
    });

    let started = Instant::now();
    assert_eq!(store.query(&query()).unwrap().len(), 1);
    let elapsed = started.elapsed();
    release.join().unwrap();
    assert!(
        elapsed >= release_after,
        "lock returned before release: {elapsed:?}"
    );
    assert!(
        elapsed < timeout,
        "released lock missed deadline: {elapsed:?}"
    );

    fs::write(dir.store_path(), b"{not-json}\n").unwrap();
    let corruption = store.query(&query()).unwrap_err();
    assert!(
        !EventStore::is_lock_timeout_error(&corruption),
        "{corruption}"
    );
    assert!(corruption.contains("corrupt"), "{corruption}");
}
