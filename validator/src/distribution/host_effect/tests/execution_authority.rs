fn repeated_digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn package() -> PackageIdentity {
    let source = SourceIdentity::new(
        repeated_digest('1'),
        repeated_digest('2'),
        "harness-ultragoal".to_owned(),
        "0.0.11".to_owned(),
        repeated_digest('3'),
        repeated_digest('4'),
    )
    .unwrap();
    PackageIdentity::new(source, repeated_digest('5'), repeated_digest('6')).unwrap()
}

fn permit_binding(
    plan: &HostCommandPlan,
    executable: &SelectedCodexExecutable,
) -> HostEffectPermitBinding {
    HostEffectPermitBinding {
        context_id: repeated_digest('1'),
        candidate_id: repeated_digest('2'),
        package_identity_sha256: repeated_digest('3'),
        journey_binding_sha256: repeated_digest('4'),
        session_issuance_sha256: repeated_digest('5'),
        lifecycle_plan_sha256: repeated_digest('6'),
        lifecycle_intent: "install".to_owned(),
        expected_pre_state_sha256: repeated_digest('7'),
        expected_post_state_sha256: repeated_digest('8'),
        rollback_policy_sha256: repeated_digest('9'),
        reconciliation_policy_sha256: repeated_digest('a'),
        host_scope_sha256: repeated_digest('b'),
        host_capability_sha256: repeated_digest('c'),
        required_capabilities_sha256: repeated_digest('d'),
        external_request_sha256: repeated_digest('e'),
        command_plan_sha256: plan.plan_sha256().to_owned(),
        argv_sha256: repeated_digest('f'),
        executable_identity_sha256: executable.binding_sha256().unwrap(),
        target_identity_sha256: repeated_digest('1'),
        target_generation: 1,
        issued_at_unix_ms: 1_000,
        expires_at_unix_ms: 2_000,
        expected_head_sha256: repeated_digest('2'),
        lifecycle_record: None,
        lifecycle_record_sha256: None,
        decision: HostEffectDecision::Authorize,
    }
}

fn in_flight_record(reservation: HostEffectReservation) -> HostEffectLedgerRecord {
    HostEffectLedgerRecord {
        reservation,
        state: HostEffectState::InFlight,
        record_sha256: repeated_digest('3'),
        prior_head: HostEffectLedgerHead::new(1, repeated_digest('4')).unwrap(),
        current_head: HostEffectLedgerHead::new(2, repeated_digest('5')).unwrap(),
        outcome_sha256: None,
    }
}

#[test]
fn ledger_transition_graph_forbids_retry_and_terminal_revival() {
    let head = HostEffectLedgerHead::new(1, repeated_digest('1')).unwrap();
    for (from, to) in [
        (HostEffectState::Reserved, HostEffectState::InFlight),
        (HostEffectState::Reserved, HostEffectState::Failed),
        (HostEffectState::InFlight, HostEffectState::Settled),
        (HostEffectState::InFlight, HostEffectState::Failed),
        (HostEffectState::InFlight, HostEffectState::Ambiguous),
        (HostEffectState::Ambiguous, HostEffectState::Settled),
        (HostEffectState::Ambiguous, HostEffectState::Failed),
    ] {
        let outcome = (to != HostEffectState::InFlight).then(|| repeated_digest('3'));
        HostEffectTransition::new(repeated_digest('2'), from, to, head.clone(), outcome).unwrap();
    }
    for (from, to) in [
        (HostEffectState::InFlight, HostEffectState::Reserved),
        (HostEffectState::Ambiguous, HostEffectState::InFlight),
        (HostEffectState::Settled, HostEffectState::InFlight),
        (HostEffectState::Failed, HostEffectState::InFlight),
        (HostEffectState::Settled, HostEffectState::Reserved),
    ] {
        assert_eq!(
            HostEffectTransition::new(repeated_digest('2'), from, to, head.clone(), None)
                .unwrap_err()
                .id(),
            HostEffectLedgerErrorId::InvalidTransition
        );
    }
    assert_eq!(
        HostEffectTransition::new(
            repeated_digest('2'),
            HostEffectState::Reserved,
            HostEffectState::InFlight,
            head.clone(),
            Some(repeated_digest('3')),
        )
        .unwrap_err()
        .id(),
        HostEffectLedgerErrorId::InvalidTransition
    );
    assert_eq!(
        HostEffectTransition::new(
            repeated_digest('2'),
            HostEffectState::InFlight,
            HostEffectState::Settled,
            head,
            None,
        )
        .unwrap_err()
        .id(),
        HostEffectLedgerErrorId::InvalidTransition
    );
}

#[cfg(unix)]
#[test]
fn pinned_executable_revalidates_exact_object_and_content() {
    let fixture = selected_test_fixture("authority-digest", b"#!/bin/sh\nexit 0\n");
    let pinned = fixture.selected().unwrap();
    assert!(is_digest(&pinned.binding_sha256().unwrap()));
    pinned.revalidate().unwrap();

    fixture.replace_contents(b"#!/bin/sh\nexit 9\n");
    assert_eq!(
        pinned.revalidate().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );
}

#[cfg(unix)]
#[test]
fn pinned_executable_rejects_named_replacement_and_hardlinks() {
    let fixture = selected_test_fixture("authority-replacement", b"#!/bin/sh\nexit 0\n");
    let pinned = fixture.selected().unwrap();
    fixture.rename_and_replace(b"#!/bin/sh\nexit 1\n");
    assert_eq!(
        pinned.revalidate().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );

    let hardlink_fixture = selected_test_fixture("authority-hardlink", b"#!/bin/sh\nexit 1\n");
    hardlink_fixture.replace_with_hard_link_from(&fixture);
    assert_eq!(
        hardlink_fixture.selected().unwrap().revalidate().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );
}
