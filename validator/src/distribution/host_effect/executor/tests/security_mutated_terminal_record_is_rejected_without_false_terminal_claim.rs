#[test]
fn security_mutated_terminal_record_is_rejected_without_false_terminal_claim() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = MutateCommitThenFailTerminalLedger { inner: &durable };
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
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::LedgerObservationRejected)
    );
    assert_eq!(
        recovery.outcome().unwrap().state,
        HostEffectState::Ambiguous
    );
    assert_eq!(
        recovery.ledger_record().unwrap().state(),
        HostEffectState::Failed
    );
    assert!(recovery.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Failed
    );
}

#[test]
fn mutation_of_recovery_permit_binding_is_detected_as_false_pass() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::ProcessSpawnFailed,
            false,
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
    let mut recovery = failure.recovery().unwrap().clone();
    assert!(recovery.verify_binding());
    recovery.substitute_permit_without_rebinding_for_test(digest('0'));
    assert!(!recovery.verify_binding());
}

#[test]
fn mutation_of_recovery_error_chain_order_length_and_members_is_detected() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::ProcessSpawnFailed,
            false,
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
    let recovery = failure.recovery().unwrap();
    let expected = [
        HostEffectExecutorErrorId::ProcessSpawnFailed,
        HostEffectExecutorErrorId::LedgerSubstitution,
    ];
    assert_eq!(recovery.originating_error_ids(), &expected);
    assert!(recovery.verify_binding());

    for replacement in [
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
        vec![
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::ProcessSpawnFailed,
        ],
        vec![HostEffectExecutorErrorId::ProcessSpawnFailed],
        vec![
            HostEffectExecutorErrorId::ProcessSpawnFailed,
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
        vec![
            HostEffectExecutorErrorId::ProcessSpawnFailed,
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::RecoveryRequired,
        ],
    ] {
        let mut mutated = recovery.clone();
        mutated.substitute_originating_error_ids_without_rebinding_for_test(replacement);
        assert!(!mutated.verify_binding());
    }
}
