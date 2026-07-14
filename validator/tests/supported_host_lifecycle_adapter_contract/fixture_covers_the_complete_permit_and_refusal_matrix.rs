#[test]
fn fixture_covers_the_complete_permit_and_refusal_matrix() {
    let cases = cases();
    assert_eq!(
        cases.schema_version,
        "SupportedHostLifecycleAdapterCases-v3"
    );
    assert_eq!(cases.binding_dimensions.len(), 24);
    assert_eq!(
        cases
            .binding_dimensions
            .iter()
            .collect::<BTreeSet<_>>()
            .len(),
        cases.binding_dimensions.len()
    );
    for required in [
        "package_identity_sha256",
        "journey_binding_sha256",
        "session_issuance_sha256",
        "lifecycle_plan_sha256",
        "host_scope_sha256",
        "host_capability_sha256",
        "required_capabilities_sha256",
        "command_plan_sha256",
        "argv_sha256",
        "executable_identity_sha256",
        "target_identity_sha256",
        "issued_at_unix_ms",
        "expected_head_sha256",
    ] {
        assert!(cases.binding_dimensions.iter().any(|row| row == required));
    }
    for required in [
        "wrong-package",
        "wrong-journey",
        "wrong-session",
        "no-effect-lifecycle-intent",
        "wrong-host-scope",
        "wrong-capability",
        "wrong-descriptor-primitive",
        "wrong-command-plan",
        "scope-plan-marketplace-mismatch",
        "repository-root-plan-mismatch",
        "repository-root-path-alias",
        "wrong-executable-object",
        "wrong-target-object",
        "untrusted-time",
        "stale-ledger-head",
        "authority-key-substitution",
        "ledger-substitution",
        "target-race",
    ] {
        assert!(cases.refusal_cases.iter().any(|row| row == required));
    }

    assert_eq!(
        cases
            .publication_identity_dimensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "effect_identity_sha256",
            "prior.name",
            "prior.kind",
            "prior.byte_length",
            "prior.mode",
            "prior.hard_links",
            "prior.content_sha256",
            "prior.data_synced",
            "next.name",
            "next.kind",
            "next.byte_length",
            "next.mode",
            "next.hard_links",
            "next.content_sha256",
            "next.data_synced",
            "temporary.name",
            "temporary.kind",
            "temporary.byte_length",
            "temporary.mode",
            "temporary.hard_links",
            "temporary.content_sha256",
            "temporary.data_synced",
        ]
    );
    assert_eq!(
        cases
            .inventory_identity_dimensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "scan_generation_before",
            "scan_generation_after",
            "target.object_generation",
            "temporary_objects[].object_generation",
            "target.exact_observed_identity",
            "temporary_objects[].exact_observed_identity",
            "current_ledger_head.generation",
            "current_ledger_head.head_sha256",
        ]
    );
    assert_eq!(
        cases
            .acknowledgement_identity_dimensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "schema_version",
            "effect_identity_sha256",
            "publication_identity_sha256",
            "ledger_head.generation",
            "ledger_head.head_sha256",
            "acknowledgement_sha256",
        ]
    );
    assert_eq!(
        cases
            .recovery_authorization_identity_dimensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "schema_version",
            "classification_sha256",
            "coordinator_binding_sha256",
            "ledger_head.generation",
            "ledger_head.head_sha256",
            "issued_at_unix_ms",
            "expires_at_unix_ms",
            "nonce_sha256",
            "authorization_sha256",
        ]
    );
    for required in [
        "classification-substitution",
        "coordinator-substitution",
        "ledger-generation-only-substitution",
        "ledger-digest-only-substitution",
        "stale-or-replayed-ledger-head",
        "mixed-ledger-head",
        "expired-authorization",
        "malformed-canonical-control",
        "unknown-canonical-control",
        "noncanonical-digest",
    ] {
        assert!(
            cases
                .recovery_authorization_refusal_cases
                .iter()
                .any(|row| row == required)
        );
    }

    let raw = source("fixtures/supported-host-lifecycle-adapter/cases.json");
    let mut unknown: serde_json::Value = serde_json::from_str(&raw).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("unknown".to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<Cases>(unknown).is_err());
    let mut missing: serde_json::Value = serde_json::from_str(&raw).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("publication_identity_dimensions");
    assert!(serde_json::from_value::<Cases>(missing).is_err());
}
