use super::*;

#[cfg(unix)]
#[test]
fn created_leaf_validation_failure_retains_leaf_and_requires_recovery() {
    let root = fs::canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!(
            "ultragoal-observability-created-leaf-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("events.jsonl");
    let store = EventStore::open_bound(&path, "context", "candidate", "source").unwrap();

    crate::observability::filesystem::fail_next_created_leaf_validation();
    assert_eq!(
        store.append(&event("retained", 1, 1, "pass")).unwrap_err(),
        "observe-store-recovery-required: created leaf cleanup cannot bind unlink to the observed identity"
    );
    assert!(path.is_file(), "ambiguous created leaf must be retained");
    assert!(fs::read(&path).unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancellation_releases_the_identity_guard() {
    let fixture = TestStore::seeded("cancelled-holder");
    let (cancel, holder) = identity_holder(&fixture.store);
    drop(cancel);
    assert!(holder
        .join()
        .expect("cancelled identity holder panicked")
        .is_err());
    assert_eq!(fixture.store.query(&fixture.query()).unwrap().len(), 1);
    fixture.remove();
}

#[test]
fn panic_drops_the_guard_and_fails_closed_without_a_timeout() {
    let fixture = TestStore::seeded("panicked-holder");
    let value = Arc::clone(&fixture.store.identity.value);
    let (ready_tx, ready_rx) = mpsc::channel();
    let holder = std::thread::spawn(move || {
        let _guard = value.lock().expect("lock panicked identity holder");
        ready_tx.send(()).expect("signal panicked identity holder");
        panic!("synthetic identity holder panic");
    });
    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("panicked identity holder did not start");
    assert!(holder.join().is_err());

    let started = Instant::now();
    assert_eq!(
        fixture.store.query(&fixture.query()).unwrap_err(),
        "observe-store-lock-failed"
    );
    assert!(
        started.elapsed() < crate::observability::locking::STORE_LOCK_TIMEOUT,
        "poisoned identity behaved like a leaked guard"
    );
    fixture.remove();
}
