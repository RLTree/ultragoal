#[test]
fn negative_clock_failure_terminal_transition_also_returns_exact_recovery() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut FailingClock,
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::StillInFlight)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::Io,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ]
    );
    assert_eq!(
        recovery.outcome().unwrap().state,
        HostEffectState::Ambiguous
    );
    assert_eq!(recovery.outcome().unwrap().command_output_sha256.len(), 1);
    assert!(recovery.has_exact_current_ledger_observation());
    assert!(recovery.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

#[test]
fn negative_preflight_ledger_substitution_after_handoff_is_identity_bearing() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let unavailable = UnavailableObservationLedger;
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &unavailable,
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
    assert_eq!(
        recovery.originating_error_id(),
        Some(HostEffectExecutorErrorId::LedgerSubstitution)
    );
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable)
    );
    assert!(!recovery.has_exact_current_ledger_observation());
    assert_eq!(recovery.outcome().unwrap().state, HostEffectState::Failed);
    assert!(recovery.verify_binding());
    assert_eq!(backend.calls, 0);
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

#[test]
fn race_committed_terminal_error_is_reobserved_as_committed_but_unverifiable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = CommitThenFailTerminalLedger { inner: &durable };
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
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));
    let recovery = failure.recovery().unwrap();
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(recovery.ledger_head(), &durable.head().unwrap());
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::TerminalCommittedButUnverifiable)
    );
    assert_eq!(
        recovery.outcome().unwrap().state,
        HostEffectState::Ambiguous
    );
    assert!(recovery.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}
