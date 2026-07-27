use super::*;

#[test]
fn identity_and_file_waits_consume_one_shared_deadline() {
    let fixture = TestStore::seeded("shared-deadline");
    let before = fixture.store_bytes();
    let file_holder = fixture.open_store_file();
    file_holder.lock().expect("hold exclusive store file lock");
    let (release_identity, identity_holder) = identity_holder(&fixture.store);
    let identity_wait = Duration::from_millis(600);
    let release = std::thread::spawn(move || {
        std::thread::sleep(identity_wait);
        release_identity.send(()).expect("release identity mutex");
    });

    let started = Instant::now();
    let error = fixture.store.query(&fixture.query()).unwrap_err();
    let elapsed = started.elapsed();
    release.join().expect("identity release panicked");
    identity_holder
        .join()
        .expect("identity holder panicked")
        .expect("identity holder cancelled");
    assert_eq!(error, "observe-store-lock-timeout");
    assert!(
        elapsed <= crate::observability::locking::STORE_LOCK_TIMEOUT + Duration::from_millis(350),
        "identity and file waits reset the deadline: {elapsed:?}"
    );
    assert_eq!(
        fixture.store_bytes(),
        before,
        "mixed contention wrote bytes"
    );

    file_holder
        .unlock()
        .expect("release exclusive store file lock");
    assert_eq!(fixture.store.query(&fixture.query()).unwrap().len(), 1);
    fixture.remove();
}

#[test]
fn identity_release_before_deadline_allows_the_store_operation() {
    let fixture = TestStore::seeded("release-before-deadline");
    let (release_identity, identity_holder) = identity_holder(&fixture.store);
    let release_after = crate::observability::locking::STORE_LOCK_TIMEOUT
        .checked_sub(Duration::from_millis(250))
        .expect("identity timeout leaves release margin");
    let release = std::thread::spawn(move || {
        std::thread::sleep(release_after);
        release_identity.send(()).expect("release identity mutex");
    });

    let started = Instant::now();
    assert_eq!(fixture.store.query(&fixture.query()).unwrap().len(), 1);
    let elapsed = started.elapsed();
    release.join().expect("identity release panicked");
    identity_holder
        .join()
        .expect("identity holder panicked")
        .expect("identity holder cancelled");
    assert!(elapsed >= release_after, "query returned before release");
    assert!(
        elapsed < crate::observability::locking::STORE_LOCK_TIMEOUT,
        "released identity missed its deadline: {elapsed:?}"
    );
    fixture.remove();
}
