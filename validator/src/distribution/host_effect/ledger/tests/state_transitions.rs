fn repeated_digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn reservation(
    head: &HostEffectLedgerHead,
    permit: char,
    nonce: char,
    semantic: char,
) -> HostEffectReservation {
    HostEffectReservation {
        issuer_id: "root-actor".to_owned(),
        ledger_id: "host-ledger".to_owned(),
        key_id: repeated_digest('a'),
        permit_id: repeated_digest(permit),
        semantic_key_sha256: repeated_digest(semantic),
        nonce_sha256: repeated_digest(nonce),
        binding_sha256: repeated_digest('b'),
        expected_head_sha256: head.head_sha256().to_owned(),
        issued_at_unix_ms: 1_000,
        expires_at_unix_ms: 2_000,
    }
}

#[test]
fn durable_ledger_reopens_and_enforces_the_exact_state_graph() {
    let fixture = LedgerFixture::new();
    let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
    let initial = ledger.head().unwrap();
    assert_eq!(initial.generation(), 0);
    let reserved = ledger
        .reserve(reservation(&initial, '1', '2', '3'))
        .unwrap();
    assert_eq!(reserved.state(), HostEffectState::Reserved);

    let head = ledger.head().unwrap();
    let in_flight = ledger
        .transition(
            HostEffectTransition::new(
                repeated_digest('1'),
                HostEffectState::Reserved,
                HostEffectState::InFlight,
                head,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(in_flight.state(), HostEffectState::InFlight);

    let head = ledger.head().unwrap();
    let ambiguous = ledger
        .transition(
            HostEffectTransition::new(
                repeated_digest('1'),
                HostEffectState::InFlight,
                HostEffectState::Ambiguous,
                head,
                Some(repeated_digest('4')),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(ambiguous.state(), HostEffectState::Ambiguous);

    let reopened = FileHostEffectLedger::open(&fixture.root, "host-ledger".to_owned()).unwrap();
    assert_eq!(
        reopened.read(&repeated_digest('1')).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
    let settled = reopened
        .transition(
            HostEffectTransition::new(
                repeated_digest('1'),
                HostEffectState::Ambiguous,
                HostEffectState::Settled,
                reopened.head().unwrap(),
                Some(repeated_digest('5')),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(settled.state(), HostEffectState::Settled);
    assert_eq!(reopened.head().unwrap().generation(), 4);
}

#[test]
fn duplicate_nonce_semantic_key_and_stale_head_fail_closed() {
    let fixture = LedgerFixture::new();
    let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
    let initial = ledger.head().unwrap();
    ledger
        .reserve(reservation(&initial, '1', '2', '3'))
        .unwrap();

    assert_eq!(
        ledger
            .reserve(reservation(&initial, '4', '5', '6'))
            .unwrap_err()
            .id(),
        HostEffectLedgerErrorId::StaleHead
    );
    let current = ledger.head().unwrap();
    assert_eq!(
        ledger
            .reserve(reservation(&current, '4', '2', '6'))
            .unwrap_err()
            .id(),
        HostEffectLedgerErrorId::Replay
    );
    assert_eq!(
        ledger
            .reserve(reservation(&current, '4', '5', '3'))
            .unwrap_err()
            .id(),
        HostEffectLedgerErrorId::Replay
    );
}

#[test]
fn concurrent_reservation_has_one_durable_winner() {
    let fixture = LedgerFixture::new();
    let ledger =
        Arc::new(FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap());
    let head = ledger.head().unwrap();
    let mut workers = Vec::new();
    for index in 0..16_u8 {
        let ledger = Arc::clone(&ledger);
        let head = head.clone();
        workers.push(thread::spawn(move || {
            let permit = char::from(b'a' + index);
            let nonce = char::from(b'a' + index);
            ledger.reserve(reservation(&head, permit, nonce, '1'))
        }));
    }
    let outcomes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|row| row.is_ok()).count(), 1);
    assert_eq!(ledger.head().unwrap().generation(), 1);
    let state_bytes = fs::read(fixture.root.join(STATE_NAME)).unwrap();
    let reopened = FileHostEffectLedger::open(&fixture.root, "host-ledger".to_owned()).unwrap();
    assert_eq!(reopened.head().unwrap().generation(), 1);
    assert_eq!(
        fs::read(fixture.root.join(STATE_NAME)).unwrap(),
        state_bytes
    );
}

#[test]
fn authenticated_state_rejects_tamper_and_same_session_rollback() {
    let fixture = LedgerFixture::new();
    let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
    let initial_bytes = fs::read(fixture.root.join(STATE_NAME)).unwrap();
    let initial = ledger.head().unwrap();
    ledger
        .reserve(reservation(&initial, '1', '2', '3'))
        .unwrap();

    overwrite(&fixture.root.join(STATE_NAME), &initial_bytes);
    assert_eq!(
        ledger.head().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );

    let mut tampered = initial_bytes;
    let index = tampered.iter().position(|byte| *byte == b'{').unwrap();
    tampered[index] = b'[';
    overwrite(&fixture.root.join(STATE_NAME), &tampered);
    let opened = FileHostEffectLedger::open(&fixture.root, "host-ledger".to_owned());
    assert!(matches!(
        opened,
        Err(error) if error.id() == HostEffectLedgerErrorId::Tampered
    ));
}

#[test]
fn read_and_head_paths_leave_ledger_bytes_and_modes_unchanged() {
    let fixture = LedgerFixture::new();
    let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
    let before = fixture.snapshot();
    let _ = ledger.head().unwrap();
    assert!(ledger.read(&repeated_digest('1')).unwrap().is_none());
    let after = fixture.snapshot();
    assert_eq!(before, after);
}
