#[test]
fn special_multiple_and_generation_races_fail_before_recovery_acceptance() {
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let head = recovery_head(7, 'a');
    let target_prior = regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), true);
    for (kind, mode) in [
        (PublicationObjectKind::Symlink, 0o120777),
        (PublicationObjectKind::Directory, 0o040700),
        (PublicationObjectKind::Fifo, 0o010600),
        (PublicationObjectKind::Socket, 0o140600),
        (PublicationObjectKind::Device, 0o020600),
        (PublicationObjectKind::Unknown, 0),
    ] {
        let special = PublicationObjectObservation::new(PublicationObjectObservationRequest {
            name: ".ledger.json.0123456789abcdef.tmp".to_owned(),
            kind,
            byte_length: 0,
            mode,
            hard_links: 1,
            content_sha256: None,
            object_generation: 10,
            data_synced: false,
        })
        .unwrap();
        assert_eq!(
            inventory(
                target_prior.clone(),
                vec![special],
                expectation.clone(),
                head.clone(),
                None,
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::UnsafeSpecialObject
        );
    }

    let first = regular_with_mode(
        ".ledger.json.0123456789abcdef.tmp",
        10,
        200,
        0o100400,
        Some(d('2')),
        true,
    );
    let second = regular_with_mode(
        ".ledger.json.fedcba9876543210.tmp",
        10,
        200,
        0o100400,
        Some(d('2')),
        true,
    );
    assert_eq!(
        inventory_at(InventoryAt {
            scan_generation_before: 10,
            scan_generation_after: 10,
            target: target_prior.clone(),
            temporary_objects: vec![first.clone(), second],
            expectation: expectation.clone(),
            current_ledger_head: head.clone(),
            acknowledgement: None,
        })
        .classify()
        .unwrap()
        .id(),
        PublicationClassificationId::MultipleTemporaryObjects
    );
    for race in [
        inventory_at(InventoryAt {
            scan_generation_before: 10,
            scan_generation_after: 11,
            target: target_prior.clone(),
            temporary_objects: vec![],
            expectation: expectation.clone(),
            current_ledger_head: head.clone(),
            acknowledgement: None,
        }),
        inventory_at(InventoryAt {
            scan_generation_before: 10,
            scan_generation_after: 10,
            target: regular_with_mode("ledger.json", 11, 100, 0o100400, Some(d('1')), true),
            temporary_objects: vec![],
            expectation: expectation.clone(),
            current_ledger_head: head.clone(),
            acknowledgement: None,
        }),
        inventory_at(InventoryAt {
            scan_generation_before: 10,
            scan_generation_after: 10,
            target: target_prior,
            temporary_objects: vec![regular_with_mode(
                ".ledger.json.0123456789abcdef.tmp",
                11,
                200,
                0o100400,
                Some(d('2')),
                true,
            )],
            expectation,
            current_ledger_head: head,
            acknowledgement: None,
        }),
    ] {
        assert_eq!(
            race.classify().unwrap().id(),
            PublicationClassificationId::ObservationRace
        );
    }
}

#[test]
fn recovery_is_proposal_only_and_requires_separate_current_authorization() {
    let fixture = Fixture::new('a');
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
    let mut issue_clock = ProbeClock {
        samples: VecDeque::from([TrustedTimeSample::new(
            "root-monotonic-clock".to_owned(),
            1,
            1,
            90_000,
        )
        .unwrap()]),
        calls: 0,
    };
    let authorization = coordinator
        .authorize_recovery(&classification, &mut issue_clock)
        .unwrap();
    let ledger_before = ledger.writes();
    let fixture_before = recursive_snapshot(&fixture.root);
    let mut proposal_clock = ProbeClock {
        samples: VecDeque::from([TrustedTimeSample::new(
            "root-monotonic-clock".to_owned(),
            1,
            2,
            90_001,
        )
        .unwrap()]),
        calls: 0,
    };
    let proposal = coordinator
        .recovery_proposal(&classification, authorization, &mut proposal_clock)
        .unwrap();
    assert_eq!(
        proposal.action(),
        RecoveryProposalAction::QuarantineTemporaryObjectForReview
    );
    assert!(!proposal.automatic_cleanup());
    assert_eq!(ledger.writes(), ledger_before);
    assert_eq!(recursive_snapshot(&fixture.root), fixture_before);
}
