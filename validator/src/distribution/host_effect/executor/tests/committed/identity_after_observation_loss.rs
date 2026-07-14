#[test]
fn committed_publication_terminal_commit_then_reobservation_loss_retains_identity() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let displaced_root = PathBuf::from(format!(
        "{}-committed-terminal-displaced",
        fixture.target_root.to_string_lossy()
    ));
    let transition_calls = Arc::new(AtomicU64::new(0));
    let committing = CommitThenDisplaceTargetLedger {
        inner: &durable,
        target_root: fixture.target_root.clone(),
        displaced_root: displaced_root.clone(),
        transition_calls: Arc::clone(&transition_calls),
    };
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &committing,
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

    assert_eq!(failure.id(), HostEffectExecutorErrorId::FalsePassReceipt);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Settled));
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::FalsePassReceipt,
            HostEffectExecutorErrorId::PathSwap,
        ]
    );
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::TerminalObserved)
    );
    assert_eq!(
        recovery.post_reservation_publication_classification(),
        Some(
            HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable
        )
    );
    assert!(recovery.has_exact_current_ledger_observation());
    assert!(!recovery.has_exact_current_publication_observation());
    assert_eq!(
        recovery
            .prior_publication_observation()
            .unwrap()
            .classify()
            .unwrap()
            .id(),
        PublicationClassificationId::CommittedBeforeAcknowledgement
    );
    assert!(recovery.outcome().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(transition_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Settled
    );
    assert!(!fixture.target_root.exists());
    assert_eq!(fs::read_dir(&displaced_root).unwrap().count(), 1);

    drop(executor);
    assert_eq!(backend.calls, 1);
    drop(lease);
    drop(observer);
    fs::remove_dir_all(displaced_root).unwrap();
}

#[test]
fn publication_failure_terminal_io_then_reobservation_loss_retains_identity() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let displaced_root = PathBuf::from(format!(
        "{}-publication-transition-io-displaced",
        fixture.target_root.to_string_lossy()
    ));
    let transition_calls = Arc::new(AtomicU64::new(0));
    let failing = DisplaceTargetAndFailTerminalLedger {
        inner: &durable,
        target_root: fixture.target_root.clone(),
        displaced_root: displaced_root.clone(),
        transition_calls: Arc::clone(&transition_calls),
    };
    set_fault(FaultPoint::BeforeTempFsync, true, |_| {});
    let mut backend = ScriptedBackend::success();
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
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::SyncFailure,
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::PathSwap,
        ]
    );
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::StillInFlight)
    );
    assert_eq!(
        recovery.post_reservation_publication_classification(),
        Some(
            HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable
        )
    );
    assert!(recovery.has_exact_current_ledger_observation());
    assert!(!recovery.has_exact_current_publication_observation());
    assert_eq!(
        recovery.ledger_record().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(
        recovery
            .prior_publication_observation()
            .unwrap()
            .classify()
            .is_ok()
    );
    assert!(recovery.observation().is_none());
    assert!(recovery.classification().is_none());
    assert!(recovery.outcome().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(transition_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(!fixture.target_root.exists());
    assert_eq!(fs::read_dir(&displaced_root).unwrap().count(), 1);

    drop(executor);
    assert_eq!(backend.calls, 1);
    drop(lease);
    drop(observer);
    fs::remove_dir_all(displaced_root).unwrap();
}
