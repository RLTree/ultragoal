use super::*;

#[test]
fn cancellation_releases_the_identity_guard() {
    let fixture = TestStore::seeded("cancelled-holder");
    let (cancel, holder) = identity_holder(&fixture.store);
    drop(cancel);
    assert!(
        holder
            .join()
            .expect("cancelled identity holder panicked")
            .is_err()
    );
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
