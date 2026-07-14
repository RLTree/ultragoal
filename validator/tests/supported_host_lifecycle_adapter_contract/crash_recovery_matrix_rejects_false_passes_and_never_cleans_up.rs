#[test]
fn crash_recovery_matrix_rejects_false_passes_and_never_cleans_up() {
    let cases = cases();
    let crash_ids = cases
        .publication_crash_cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    for expected in [
        "before-temp-write",
        "during-temp-fsync",
        "before-rename",
        "after-rename",
        "before-acknowledgement",
        "orphan-temp-after-publication",
    ] {
        assert!(crash_ids.contains(expected));
    }
    assert!(
        cases
            .publication_crash_cases
            .iter()
            .all(|case| !case.target.is_empty()
                && !case.temp.is_empty()
                && !case.expected.is_empty())
    );
    assert!(
        cases
            .unsafe_publication_cases
            .iter()
            .any(|row| row == "fifo-temp")
    );
    assert!(
        cases
            .unsafe_publication_cases
            .iter()
            .any(|row| row == "socket-temp")
    );
    for required in [
        "scan-generation-race",
        "target-object-generation-race",
        "temporary-object-generation-race",
    ] {
        assert!(
            cases
                .unsafe_publication_cases
                .iter()
                .any(|row| row == required)
        );
    }
    for required in [
        "prior-byte-length-drift",
        "prior-writable-mode",
        "prior-link-count-drift",
        "prior-unsynced",
        "next-byte-length-drift",
        "next-writable-mode",
        "next-link-count-drift",
        "next-unsynced",
        "temporary-name-substitution",
        "temporary-byte-length-drift",
        "temporary-writable-mode",
        "temporary-link-count-drift",
        "temporary-synced-corrupt-content",
        "target-special-object",
        "multiple-temporary-objects",
        "scan-generation-race",
        "target-object-generation-race",
        "temporary-object-generation-race",
    ] {
        assert!(
            cases
                .exactness_refusal_cases
                .iter()
                .any(|row| row == required)
        );
    }
    for required in [
        "bare-boolean-acknowledgement",
        "unknown-field",
        "missing-field",
        "extra-field",
        "duplicate-field",
        "reordered-fields",
        "surrounding-whitespace",
        "noncanonical-digest",
        "stale-ledger-generation",
        "stale-ledger-head",
        "foreign-effect-identity",
        "foreign-publication-identity",
        "target-mode-drift",
        "target-byte-length-drift",
        "target-unsynced",
        "orphan-temporary-object",
    ] {
        assert!(
            cases
                .canonical_acknowledgement_refusal_cases
                .iter()
                .any(|row| row == required)
        );
    }
    assert!(cases.false_pass_cases.iter().all(|case| {
        !case.acknowledgement.is_empty()
            && !case.substitution.is_empty()
            && !case.target.is_empty()
            && case.expected == "false-pass-receipt"
    }));
    assert!(cases.recovery_contract.classification_is_read_only);
    assert!(cases.recovery_contract.expected_prior_object_is_exact);
    assert!(cases.recovery_contract.expected_next_object_is_exact);
    assert!(cases.recovery_contract.expected_temporary_object_is_exact);
    assert!(
        cases
            .recovery_contract
            .writable_expected_targets_are_rejected
    );
    assert!(!cases.recovery_contract.acknowledgement_is_bare_boolean);
    assert!(
        cases
            .recovery_contract
            .acknowledgement_requires_canonical_bytes
    );
    assert!(
        cases
            .recovery_contract
            .acknowledgement_binds_effect_identity
    );
    assert!(
        cases
            .recovery_contract
            .acknowledgement_binds_publication_identity
    );
    assert!(
        cases
            .recovery_contract
            .acknowledgement_binds_current_ledger_head
    );
    assert!(
        !cases
            .recovery_contract
            .acknowledgement_is_recovery_authorization
    );
    assert!(
        cases
            .recovery_contract
            .unknown_missing_extra_or_ambiguous_fields_are_rejected
    );
    assert!(cases.recovery_contract.separate_authorization_required);
    assert!(cases.recovery_contract.authorization_binds_coordinator);
    assert!(
        cases
            .recovery_contract
            .authorization_binds_current_ledger_head
    );
    assert!(
        cases
            .recovery_contract
            .authorization_binds_ledger_generation
    );
    assert!(cases.recovery_contract.authorization_binds_ledger_digest);
    assert!(
        cases
            .recovery_contract
            .authorization_rejects_stale_or_replayed_head
    );
    assert!(
        cases
            .recovery_contract
            .authorization_rejects_malformed_or_unknown_canonical_controls
    );
    assert!(
        cases
            .recovery_contract
            .proposal_proof_binds_authorized_ledger_head
    );
    assert!(cases.recovery_contract.authorization_expires);
    assert!(!cases.recovery_contract.proposal_performs_cleanup);
    assert!(!cases.recovery_contract.automatic_ambiguous_retry);

    let recovery = source("validator/src/distribution/host_effect/lifecycle/recovery.rs");
    for required in [
        "ExpectedPublicationObjectIdentity",
        "PublicationExpectation",
        "PublicationAcknowledgementIdentity",
        "from_canonical_json",
        "serde(deny_unknown_fields)",
        "harness-ultragoal.prepublication-recovery-classification.v2",
        "harness-ultragoal.host-effect-recovery-authorization.v2",
        "ledger_head: HostEffectLedgerHead",
        "ledger_head: &'a HostEffectLedgerHead",
        "authorized_ledger_head: &'a HostEffectLedgerHead",
        "&authorization.ledger_head != head",
        "automatic_cleanup: false",
    ] {
        assert!(
            recovery.contains(required),
            "missing recovery control: {required}"
        );
    }
    for removed in [
        "acknowledgement_present: bool",
        "expected_prior_sha256",
        "expected_next_sha256",
        "expected_next_length",
        "ledger_head_sha256",
    ] {
        assert!(
            !recovery.contains(removed),
            "digest-only or boolean recovery input remains: {removed}"
        );
    }
    for forbidden in [
        "std::fs",
        "remove_file",
        "remove_dir",
        "rename(",
        "unlink",
        "Command::new",
        "std::process",
    ] {
        assert!(
            !recovery.contains(forbidden),
            "forbidden recovery effect: {forbidden}"
        );
    }
}
