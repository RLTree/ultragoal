#[test]
fn every_complete_permit_binding_dimension_changes_the_canonical_binding() {
    let fixture = Fixture::new('9');
    let ledger = RecordingLedger::new(0, d('a'));
    let (_coordinator, request, _custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let baseline = request
        .derive_binding(100_000, 100_001, &ledger.observed_head())
        .unwrap();
    let baseline_digest = digest(&serde_json::to_vec(&baseline).unwrap());
    let mutations: &[fn(&mut super::super::HostEffectPermitBinding)] = &[
        |row| row.context_id = d('b'),
        |row| row.candidate_id = d('b'),
        |row| row.package_identity_sha256 = d('b'),
        |row| row.journey_binding_sha256 = d('b'),
        |row| row.session_issuance_sha256 = d('b'),
        |row| row.lifecycle_plan_sha256 = d('b'),
        |row| row.lifecycle_intent = "authorized-rollback".to_owned(),
        |row| row.expected_pre_state_sha256 = d('b'),
        |row| row.expected_post_state_sha256 = d('b'),
        |row| row.rollback_policy_sha256 = d('b'),
        |row| row.reconciliation_policy_sha256 = d('b'),
        |row| row.host_scope_sha256 = d('b'),
        |row| row.host_capability_sha256 = d('b'),
        |row| row.required_capabilities_sha256 = d('b'),
        |row| row.external_request_sha256 = d('b'),
        |row| row.command_plan_sha256 = d('b'),
        |row| row.argv_sha256 = d('b'),
        |row| row.executable_identity_sha256 = d('b'),
        |row| row.target_identity_sha256 = d('b'),
        |row| row.target_generation += 1,
        |row| row.issued_at_unix_ms += 1,
        |row| row.expires_at_unix_ms += 1,
        |row| row.expected_head_sha256 = d('b'),
        |row| row.decision = super::super::HostEffectDecision::Refuse,
    ];
    assert_eq!(mutations.len(), 24);
    let authority = HostEffectAuthority::generate(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
    )
    .unwrap();
    authority.issue(baseline.clone()).unwrap();
    for mutate in mutations {
        let mut changed = baseline.clone();
        mutate(&mut changed);
        assert_ne!(
            digest(&serde_json::to_vec(&changed).unwrap()),
            baseline_digest
        );
    }
}

#[test]
fn exact_publication_identities_classify_every_recoverable_state() {
    let prior = d('1');
    let next = d('2');
    let head = recovery_head(7, 'a');
    let prior_expectation = publication_expectation(expected_regular(
        "ledger.json",
        100,
        0o100400,
        prior.clone(),
        true,
    ));
    let missing_expectation = publication_expectation(expected_missing("ledger.json"));
    let target_prior = regular_with_mode("ledger.json", 10, 100, 0o100400, Some(prior), true);
    let target_next = regular_with_mode("ledger.json", 10, 200, 0o100400, Some(next.clone()), true);
    let target_missing =
        PublicationObjectObservation::missing("ledger.json".to_owned(), 10).unwrap();
    let temp_empty = regular_with_mode(
        ".ledger.json.0123456789abcdef.tmp",
        10,
        0,
        0o100400,
        None,
        false,
    );
    let temp_unflushed = regular_with_mode(
        ".ledger.json.0123456789abcdef.tmp",
        10,
        200,
        0o100400,
        Some(next.clone()),
        false,
    );
    let temp_synced = regular_with_mode(
        ".ledger.json.0123456789abcdef.tmp",
        10,
        200,
        0o100400,
        Some(next),
        true,
    );
    let acknowledgement =
        PublicationAcknowledgementIdentity::new(&prior_expectation, &head).unwrap();

    let cases = [
        (
            inventory(
                target_missing.clone(),
                vec![],
                missing_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::CleanPriorState,
        ),
        (
            inventory(
                target_prior.clone(),
                vec![],
                prior_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::CleanPriorState,
        ),
        (
            inventory(
                target_missing,
                vec![temp_empty],
                missing_expectation,
                head.clone(),
                None,
            ),
            PublicationClassificationId::InterruptedBeforeTempWrite,
        ),
        (
            inventory(
                target_prior.clone(),
                vec![temp_unflushed],
                prior_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::InterruptedDuringTempFsync,
        ),
        (
            inventory(
                target_prior.clone(),
                vec![temp_synced.clone()],
                prior_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::InterruptedBeforeRename,
        ),
        (
            inventory(
                target_next.clone(),
                vec![],
                prior_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::CommittedBeforeAcknowledgement,
        ),
        (
            inventory(
                target_next.clone(),
                vec![],
                prior_expectation.clone(),
                head.clone(),
                Some(acknowledgement),
            ),
            PublicationClassificationId::AcknowledgedCommitted,
        ),
        (
            inventory(
                target_next,
                vec![temp_synced],
                prior_expectation,
                head,
                None,
            ),
            PublicationClassificationId::OrphanedTemporaryObject,
        ),
    ];
    for (observation, expected) in cases {
        assert_eq!(observation.classify().unwrap().id(), expected);
    }
}
