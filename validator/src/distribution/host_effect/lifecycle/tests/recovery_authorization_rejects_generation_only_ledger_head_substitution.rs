#[test]
fn recovery_authorization_rejects_generation_only_ledger_head_substitution() {
    let ledger = RecordingLedger::new(0, d('a'));
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        &ledger,
    )
    .unwrap();
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let classification = inventory(
        regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), true),
        vec![regular_with_mode(
            ".ledger.json.0123456789abcdef.tmp",
            10,
            200,
            0o100400,
            Some(d('2')),
            true,
        )],
        expectation,
        recovery_head(0, 'a'),
        None,
    )
    .classify()
    .unwrap();

    let mut issue_clock = one_sample_clock(95_000, 1);
    let authorization = coordinator
        .authorize_recovery(&classification, &mut issue_clock)
        .unwrap();
    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(1, d('a')).unwrap();
    let mut proposal_clock = one_sample_clock(95_001, 2);
    assert_eq!(
        coordinator
            .recovery_proposal(&classification, authorization, &mut proposal_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );
    assert_eq!(ledger.writes(), 0);
}

#[test]
fn recovery_authorization_rejects_classification_head_and_expiry_substitution() {
    let ledger = RecordingLedger::new(0, d('a'));
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        &ledger,
    )
    .unwrap();
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let before_rename = inventory(
        regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), true),
        vec![regular_with_mode(
            ".ledger.json.0123456789abcdef.tmp",
            10,
            200,
            0o100400,
            Some(d('2')),
            true,
        )],
        expectation.clone(),
        recovery_head(0, 'a'),
        None,
    )
    .classify()
    .unwrap();
    let committed = inventory(
        regular_with_mode("ledger.json", 10, 200, 0o100400, Some(d('2')), true),
        vec![],
        expectation,
        recovery_head(0, 'a'),
        None,
    )
    .classify()
    .unwrap();

    let mut issue_clock = one_sample_clock(100_000, 1);
    let authorization = coordinator
        .authorize_recovery(&before_rename, &mut issue_clock)
        .unwrap();
    let mut proposal_clock = one_sample_clock(100_001, 2);
    assert_eq!(
        coordinator
            .recovery_proposal(&committed, authorization, &mut proposal_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );

    let mut issue_clock = one_sample_clock(110_000, 3);
    let authorization = coordinator
        .authorize_recovery(&before_rename, &mut issue_clock)
        .unwrap();
    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(0, d('b')).unwrap();
    let mut proposal_clock = one_sample_clock(110_001, 4);
    assert_eq!(
        coordinator
            .recovery_proposal(&before_rename, authorization, &mut proposal_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );

    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(0, d('a')).unwrap();
    let mut issue_clock = one_sample_clock(115_000, 5);
    let authorization = coordinator
        .authorize_recovery(&before_rename, &mut issue_clock)
        .unwrap();
    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(1, d('b')).unwrap();
    let mut proposal_clock = one_sample_clock(115_001, 6);
    assert_eq!(
        coordinator
            .recovery_proposal(&before_rename, authorization, &mut proposal_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );

    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(0, d('a')).unwrap();
    let mut issue_clock = one_sample_clock(120_000, 7);
    let authorization = coordinator
        .authorize_recovery(&before_rename, &mut issue_clock)
        .unwrap();
    let mut expired_clock = one_sample_clock(180_001, 8);
    assert_eq!(
        coordinator
            .recovery_proposal(&before_rename, authorization, &mut expired_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );
    assert_eq!(ledger.writes(), 0);
}

fn one_sample_clock(unix_ms: u64, sequence: u64) -> ProbeClock {
    ProbeClock {
        samples: VecDeque::from([TrustedTimeSample::new(
            "root-monotonic-clock".to_owned(),
            1,
            sequence,
            unix_ms,
        )
        .unwrap()]),
        calls: 0,
    }
}

fn regular_with_mode(
    name: &str,
    generation: u64,
    byte_length: u64,
    mode: u32,
    content_sha256: Option<String>,
    data_synced: bool,
) -> PublicationObjectObservation {
    PublicationObjectObservation::new(PublicationObjectObservationRequest {
        name: name.to_owned(),
        kind: PublicationObjectKind::Regular,
        byte_length,
        mode,
        hard_links: 1,
        content_sha256,
        object_generation: generation,
        data_synced,
    })
    .unwrap()
}
