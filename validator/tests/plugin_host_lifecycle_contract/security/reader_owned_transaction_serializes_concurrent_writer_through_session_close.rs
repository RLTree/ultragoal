#[test]
fn reader_owned_transaction_serializes_concurrent_writer_through_session_close() {
    let fixture = Fixture::new("transaction-concurrent-writer");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 17);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(LockedSurfaceState {
        reader: Reader::complete(&bundle, &host, session.binding()),
        generation: 0,
    }));
    let writer_ready = std::sync::Arc::new(std::sync::Barrier::new(2));
    let writer_state = std::sync::Arc::clone(&shared);
    let writer_barrier = std::sync::Arc::clone(&writer_ready);
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        let mut state = writer_state.lock().unwrap();
        state.reader.marketplace = Some(b"POST_CLOSE_WRITER_CANARY".to_vec());
        state.generation += 1;
    });
    let mut reader = LockedSurfaceReader {
        state: std::sync::Arc::clone(&shared),
        writer_ready,
    };

    assert!(session.capture_and_verify(&mut reader).is_ok());
    writer.join().unwrap();
    assert_eq!(shared.lock().unwrap().generation, 1);
    let mut replay = Reader::complete(&bundle, &host, session.binding());
    assert_eq!(
        session.capture_and_verify(&mut replay).unwrap_err().id(),
        HostLifecycleErrorId::SessionStateRejected
    );
    assert_eq!(fixture.tree(), before);
}

#[test]
fn concurrent_capture_and_verify_has_one_winner_and_one_closed_loser() {
    let fixture = Fixture::new("concurrent-observation-transaction");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 12);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let reader = Reader::complete(&bundle, &host, session.binding());
    let before_tree = fixture.tree();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(session));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut handles = Vec::new();
    for mut reader in [reader.clone(), reader] {
        let shared = std::sync::Arc::clone(&shared);
        let barrier = std::sync::Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            shared
                .lock()
                .unwrap()
                .capture_and_verify(&mut reader)
                .map(|_| ())
                .map_err(|error| error.id())
        }));
    }
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .copied()
            .collect::<Vec<_>>(),
        vec![HostLifecycleErrorId::SessionStateRejected]
    );
    assert_eq!(fixture.tree(), before_tree);
}
