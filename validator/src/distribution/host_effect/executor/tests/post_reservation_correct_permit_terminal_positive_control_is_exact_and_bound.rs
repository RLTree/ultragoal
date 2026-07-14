#[test]
fn post_reservation_correct_permit_terminal_positive_control_is_exact_and_bound() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let terminal = recovering_terminal_record(&durable, &effect);
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        terminal.current_head().clone(),
        terminal.clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![HostEffectExecutorErrorId::Timeout],
    );
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));
    let recovery = failure.recovery().unwrap();
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(recovery.ledger_record().unwrap(), &terminal);
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::TerminalObserved)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[HostEffectExecutorErrorId::Timeout]
    );
    assert!(recovery.outcome().is_none());
    assert!(recovery.observation().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 2);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn post_reservation_correct_permit_rejected_positive_control_is_exact_and_bound() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let mut rejected = effect.record().clone();
    rejected.state = HostEffectState::Reserved;
    rejected.outcome_sha256 = None;
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        rejected.current_head().clone(),
        rejected.clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![HostEffectExecutorErrorId::Timeout],
    );
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(recovery.ledger_record().unwrap(), &rejected);
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::ObservationRejected)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[HostEffectExecutorErrorId::Timeout]
    );
    assert!(recovery.outcome().is_none());
    assert!(recovery.observation().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 2);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn post_reservation_observation_error_falls_back_bound_unavailable_and_preserves_chain() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let ledger = PostReservationObservationLedger::read_error(
        effect.permit().permit_id().to_owned(),
        effect.record().current_head().clone(),
        effect.record().clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![HostEffectExecutorErrorId::Timeout],
    );
    assert_bound_unavailable_post_reservation(
        &failure,
        HostEffectExecutorErrorId::RecoveryRequired,
        &effect,
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 3);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 3);
}

#[test]
fn wrong_permit_post_reservation_replay_preflight_route_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let replay_record = recovering_terminal_record(&durable, &effect);
    let wrong_permit_record = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::InFlight,
    );
    let ledger = ReplayThenWrongPermitObservationLedger {
        expected_permit_id: effect.permit().permit_id().to_owned(),
        replay_record,
        observed_head: wrong_permit_record.current_head().clone(),
        wrong_permit_record,
        head_calls: AtomicU64::new(0),
        read_calls: AtomicU64::new(0),
    };
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &ledger,
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
    assert_bound_unavailable_post_reservation(
        &failure,
        HostEffectExecutorErrorId::Replay,
        &effect,
        &[HostEffectExecutorErrorId::Replay],
    );
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 8);
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
    drop(executor);
    assert_eq!(backend.calls, 0);
}
