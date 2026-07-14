#[test]
fn positive_actual_publication_fsync_rename_ledger_and_ack_transaction_settles() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend::success();
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let expected_effect_identity = executor.effect_identity(&capability(), &effect).unwrap();
    let receipt = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap();
    assert_eq!(
        receipt.publication_classification().id(),
        PublicationClassificationId::AcknowledgedCommitted
    );
    assert_eq!(receipt.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(receipt.outcome().state, HostEffectState::Settled);
    assert_eq!(receipt.command_output_sha256().len(), 1);
    assert_eq!(
        PublicationAcknowledgementIdentity::from_canonical_json(receipt.acknowledgement_json())
            .unwrap(),
        *receipt.acknowledgement()
    );
    let terminal = ledger.read(&permit_id).unwrap().unwrap();
    assert_eq!(terminal.state(), HostEffectState::Settled);
    assert_eq!(terminal.current_head(), receipt.terminal_ledger_head());
    let entries = fs::read_dir(&fixture.target_root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].starts_with("effect-"));
    let metadata = fs::symlink_metadata(fixture.target_root.join(&entries[0])).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.permissions().mode() & 0o777, 0o400);
    assert_eq!(metadata.nlink(), 1);

    let replay = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(replay.id(), HostEffectExecutorErrorId::Replay);
    assert_eq!(replay.terminal_state(), Some(HostEffectState::Settled));
    let replay_recovery = replay.recovery().unwrap();
    assert_eq!(replay_recovery.permit_id(), permit_id);
    assert!(replay_recovery.has_exact_current_ledger_observation());
    assert_eq!(
        replay_recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::TerminalObserved)
    );
    assert_eq!(
        replay_recovery.post_reservation_publication_classification(),
        Some(HostEffectPostReservationPublicationClassification::NoPublicationEvidence)
    );
    assert_eq!(
        replay_recovery.originating_error_ids(),
        &[HostEffectExecutorErrorId::Replay]
    );
    assert!(replay_recovery.verify_binding());
    drop(executor);
    assert_eq!(backend.calls, 1);
}

#[test]
fn negative_started_backend_terminal_failure_before_mutation_returns_exact_recovery() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::Timeout,
            true,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let capability = capability();
    let expected_effect_identity = executor.effect_identity(&capability, &effect).unwrap();
    let failure = executor
        .execute_authorized_for_test(
            &capability,
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_ne!(failure.id(), HostEffectExecutorErrorId::LedgerSubstitution);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(recovery.ledger_head(), &durable.head().unwrap());
    assert_eq!(
        recovery.ledger_record().unwrap(),
        &durable.read(&permit_id).unwrap().unwrap()
    );
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::StillInFlight)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ]
    );
    let outcome = recovery.outcome().unwrap();
    assert_eq!(outcome.permit_id, permit_id);
    assert_eq!(outcome.state, HostEffectState::Ambiguous);
    assert!(outcome.observed_post_state_sha256.is_none());
    assert!(recovery.binding_sha256().unwrap().starts_with("sha256:"));
    assert!(recovery.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}
