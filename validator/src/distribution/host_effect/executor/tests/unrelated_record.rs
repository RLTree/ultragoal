fn unrelated_record(
    fixture: &Fixture,
    ledger: &FileHostEffectLedger,
    target: &crate::distribution::host_effect::lifecycle::ObservedTargetIdentity,
    state: HostEffectState,
) -> HostEffectLedgerRecord {
    let authority =
        HostEffectAuthority::generate("unrelated-root".to_owned(), "fixture-ledger".to_owned())
            .unwrap();
    let (_permit, reservation) = authority
        .issue(permit_binding(fixture, ledger, target, '0'))
        .unwrap();
    let reserved = ledger.reserve(reservation).unwrap();
    if state == HostEffectState::Reserved {
        return reserved;
    }
    let in_flight = ledger
        .transition(
            HostEffectTransition::new(
                reserved.reservation().permit_id().to_owned(),
                HostEffectState::Reserved,
                HostEffectState::InFlight,
                reserved.current_head().clone(),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    if state == HostEffectState::InFlight {
        return in_flight;
    }
    ledger
        .transition(
            HostEffectTransition::new(
                in_flight.reservation().permit_id().to_owned(),
                HostEffectState::InFlight,
                state,
                in_flight.current_head().clone(),
                Some(digest('d')),
            )
            .unwrap(),
        )
        .unwrap()
}

fn recovering_terminal_record(
    ledger: &FileHostEffectLedger,
    effect: &AuthorizedHostEffect,
) -> HostEffectLedgerRecord {
    ledger
        .transition(
            HostEffectTransition::new(
                effect.permit().permit_id().to_owned(),
                HostEffectState::InFlight,
                HostEffectState::Ambiguous,
                effect.record().current_head().clone(),
                Some(digest('d')),
            )
            .unwrap(),
        )
        .unwrap()
}

fn direct_post_reservation_failure(
    target: ConfinedHostEffectTarget,
    ledger: &PostReservationObservationLedger,
    effect: &AuthorizedHostEffect,
    originating_error_ids: Vec<HostEffectExecutorErrorId>,
) -> HostEffectExecutorFailure {
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        ledger,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let effect_identity = executor.effect_identity(&capability(), effect).unwrap();
    let failure = executor.post_reservation_recovery_failure(
        effect,
        &effect_identity,
        HostEffectExecutorErrorId::RecoveryRequired,
        originating_error_ids,
        None,
    );
    drop(executor);
    assert_eq!(backend.calls, 0);
    failure
}

fn assert_bound_unavailable_post_reservation(
    failure: &HostEffectExecutorFailure,
    returned_error_id: HostEffectExecutorErrorId,
    effect: &AuthorizedHostEffect,
    expected_error_ids: &[HostEffectExecutorErrorId],
) {
    assert_eq!(failure.id(), returned_error_id);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.permit_id(), effect.permit().permit_id());
    assert_eq!(recovery.ledger_head(), effect.record().current_head());
    assert_eq!(recovery.ledger_record().unwrap(), effect.record());
    assert!(!recovery.has_exact_current_ledger_observation());
    assert!(!recovery.has_exact_current_publication_observation());
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::ObservationUnavailable)
    );
    assert_eq!(
        recovery.post_reservation_publication_classification(),
        Some(HostEffectPostReservationPublicationClassification::NoPublicationEvidence)
    );
    assert_eq!(recovery.originating_error_ids(), expected_error_ids);
    assert!(recovery.terminal_classification().is_none());
    assert!(recovery.outcome().is_none());
    assert!(recovery.observation().is_none());
    assert!(recovery.prior_publication_observation().is_none());
    assert!(recovery.publication_identity_sha256().is_none());
    assert!(recovery.classification().is_none());
    assert!(recovery.verify_binding());

    let mut substituted = recovery.clone();
    substituted.substitute_permit_without_rebinding_for_test(digest('0'));
    assert!(!substituted.verify_binding());
}

#[test]
fn wrong_permit_post_reservation_still_in_flight_observation_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let wrong_record = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::InFlight,
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
