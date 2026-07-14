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
        store.export_explicit(
            &query(),
            true,
            true,
            Duration::from_secs(2),
            Some(&mut adapter),
        )
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
pub(crate) fn same_process_identity_contention_times_out_every_store_path_without_hidden_writes() {
    let dir = TestDir::new("identity-lock-deadline");
    let store = store(&dir);
    assert!(store.append(&event("seed", 1, 1, "pass")).unwrap());
    initialize_git(dir.path());
    let before = tree_snapshot(dir.path());
    let before_status = git_status(dir.path());
    let before_store = fs::read(dir.store_path()).unwrap();

    assert_identity_lock_timeout(&store, || store.query(&query()));
    assert_identity_lock_timeout(&store, || store.append(&event("blocked", 2, 2, "fail")));
    assert_identity_lock_timeout(&store, || store.clear());
    assert_identity_lock_timeout(&store, || store.recover_truncated_tail());

    let mut adapter = MockAdapter::new(AdapterMode::Ok);
    assert_identity_lock_timeout(&store, || {
        store.export_explicit(
            &query(),
            true,
            true,
            Duration::from_secs(2),
            Some(&mut adapter),
        )
    });
    assert_eq!(adapter.export_calls, 0, "timeout reached export adapter");
    assert_eq!(
        tree_snapshot(dir.path()),
        before,
        "identity contention wrote store or tree bytes"
    );
    assert_eq!(
        git_status(dir.path()),
        before_status,
        "identity contention changed Git status"
    );
    assert_eq!(
        fs::read(dir.store_path()).unwrap(),
        before_store,
        "identity contention changed retained store bytes"
    );
    assert_eq!(store.query(&query()).unwrap().len(), 1);
}

#[test]
pub(crate) fn identity_wait_and_file_wait_consume_one_shared_monotonic_budget() {
    let dir = TestDir::new("mixed-identity-file-deadline");
    let store = store(&dir);
    assert!(store.append(&event("seed", 1, 1, "pass")).unwrap());
    let before = tree_snapshot(dir.path());
    let file_holder = locked_exclusive(&dir);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let holder_store = store.clone();
    let identity_holder =
        std::thread::spawn(move || holder_store.hold_identity_mutex_for_test(ready_tx, release_rx));
    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("identity holder did not start");
    let identity_wait = Duration::from_millis(600);
    let release_identity = std::thread::spawn(move || {
        std::thread::sleep(identity_wait);
        release_tx.send(()).unwrap();
    });

    let started = Instant::now();
    let result = store.query(&query());
    let elapsed = started.elapsed();
    release_identity.join().unwrap();
    identity_holder.join().unwrap().unwrap();
    assert_timeout_result(result, elapsed);
    assert!(
        elapsed <= lock_timeout() + Duration::from_millis(350),
        "identity and file waits reset the budget: {elapsed:?}"
    );
    assert_eq!(
        tree_snapshot(dir.path()),
        before,
        "mixed contention wrote tree bytes"
    );

    file_holder.unlock().unwrap();
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
            .export_explicit(
                &query(),
                true,
                true,
                Duration::from_secs(2),
                Some(&mut adapter),
            )
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

#[test]
pub(crate) fn identity_release_before_deadline_succeeds_and_cancelled_holder_leaves_no_lock() {
    let dir = TestDir::new("identity-release-before-deadline");
    let store = store(&dir);
    assert!(store.append(&event("seed", 1, 1, "pass")).unwrap());
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let holder_store = store.clone();
    let holder =
        std::thread::spawn(move || holder_store.hold_identity_mutex_for_test(ready_tx, release_rx));
    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("identity holder did not start");
    let release_after = lock_timeout()
        .checked_sub(Duration::from_millis(250))
        .expect("identity timeout leaves release margin");
    let release = std::thread::spawn(move || {
        std::thread::sleep(release_after);
        release_tx.send(()).unwrap();
    });

    let started = Instant::now();
    assert_eq!(store.query(&query()).unwrap().len(), 1);
    let elapsed = started.elapsed();
    release.join().unwrap();
    holder.join().unwrap().unwrap();
    assert!(
        elapsed >= release_after,
        "identity lock returned before release: {elapsed:?}"
    );
    assert!(
        elapsed < lock_timeout(),
        "released identity lock missed deadline: {elapsed:?}"
    );

    let (ready_tx, ready_rx) = mpsc::channel();
    let (cancel_tx, cancel_rx) = mpsc::channel();
    let holder_store = store.clone();
    let cancelled =
        std::thread::spawn(move || holder_store.hold_identity_mutex_for_test(ready_tx, cancel_rx));
    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("cancelled holder did not start");
    drop(cancel_tx);
    assert!(cancelled.join().unwrap().is_err());
    assert_eq!(
        store.query(&query()).unwrap().len(),
        1,
        "cancelled holder leaked its identity mutex"
    );
}
