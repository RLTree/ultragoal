fn assert_unrelated_head_advance_returns_bound_unavailable_recovery(
    observation_mode: RecoveryObservationMode,
    commit_terminal: bool,
    repeat_advance: bool,
) {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let reservation_head = effect.record().current_head().clone();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let advancing = AdvanceUnrelatedPermitThenFailTerminalLedger {
        inner: &durable,
        authority: HostEffectAuthority::generate(
            "unrelated-root".to_owned(),
            "fixture-ledger".to_owned(),
        )
        .unwrap(),
        unrelated_binding: permit_binding(&fixture, &durable, &target_identity, '0'),
        stale_head: reservation_head.clone(),
        commit_terminal,
        repeat_advance,
        observation_mode,
        advanced: AtomicBool::new(false),
        unrelated_record: Mutex::new(None),
    };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::Timeout,
            true,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &advancing,
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
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(recovery.ledger_head(), &reservation_head);
    assert_eq!(recovery.ledger_record().unwrap(), effect.record());
    assert!(!recovery.has_exact_current_ledger_observation());
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ]
    );
    assert_eq!(
        recovery.outcome().unwrap().state,
        HostEffectState::Ambiguous
    );
    assert!(recovery.observation().is_none());
    assert!(recovery.prior_publication_observation().is_none());
    assert!(recovery.publication_identity_sha256().is_none());
    assert!(recovery.classification().is_none());
    assert!(recovery.verify_binding());

    let global_head_after = durable.head().unwrap();
    assert_ne!(global_head_after, reservation_head);
    let mut mismatched = recovery.clone();
    mismatched.substitute_ledger_head_without_rebinding_for_test(global_head_after);
    assert!(!mismatched.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        if commit_terminal {
            HostEffectState::Ambiguous
        } else {
            HostEffectState::InFlight
        }
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
    drop(executor);
    assert_eq!(backend.calls, 1);
}

#[test]
fn race_unrelated_permit_global_head_advance_with_still_in_flight_record_is_unavailable_and_bound()
{
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::Current,
        false,
        false,
    );
}

#[test]
fn race_repeated_unrelated_head_advances_after_terminal_commit_withhold_terminal_classification() {
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::Current,
        true,
        true,
    );
}

#[test]
fn negative_unrelated_head_advance_record_substitution_is_unavailable_and_bound() {
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::SubstituteUnrelatedRecord,
        false,
        false,
    );
}

#[test]
fn negative_head_rollback_after_terminal_and_unrelated_advance_is_unavailable_and_bound() {
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::StaleHead,
        true,
        false,
    );
}

#[test]
fn negative_observation_error_after_unrelated_head_advance_is_unavailable_and_bound() {
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::Error,
        false,
        false,
    );
}
