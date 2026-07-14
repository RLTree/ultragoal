#[test]
fn wrong_permit_post_reservation_terminal_observation_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let wrong_record = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::Ambiguous,
    );
    assert_ne!(
        wrong_record.reservation().permit_id(),
        effect.permit().permit_id()
    );
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        wrong_record.current_head().clone(),
        wrong_record,
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
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
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn wrong_permit_post_reservation_rejected_observation_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let wrong_record = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::Reserved,
    );
    assert_ne!(
        wrong_record.reservation().permit_id(),
        effect.permit().permit_id()
    );
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        wrong_record.current_head().clone(),
        wrong_record,
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
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
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn post_reservation_right_record_wrong_head_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let unrelated = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::Reserved,
    );
    assert_ne!(unrelated.current_head(), effect.record().current_head());
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        unrelated.current_head().clone(),
        effect.record().clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
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
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn post_reservation_correct_permit_still_in_flight_positive_control_is_exact_and_bound() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let ledger = PostReservationObservationLedger::stable(
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
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::StillInFlight)
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
