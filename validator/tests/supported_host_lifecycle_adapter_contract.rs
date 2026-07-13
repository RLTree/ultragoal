use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cases {
    schema_version: String,
    binding_dimensions: Vec<String>,
    publication_identity_dimensions: Vec<String>,
    inventory_identity_dimensions: Vec<String>,
    acknowledgement_identity_dimensions: Vec<String>,
    recovery_authorization_identity_dimensions: Vec<String>,
    recovery_authorization_refusal_cases: Vec<String>,
    refusal_cases: Vec<String>,
    darwin: Darwin,
    publication_crash_cases: Vec<CrashCase>,
    unsafe_publication_cases: Vec<String>,
    exactness_refusal_cases: Vec<String>,
    canonical_acknowledgement_refusal_cases: Vec<String>,
    false_pass_cases: Vec<FalsePassCase>,
    recovery_contract: RecoveryContract,
    public_api_contract: PublicApiContract,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Darwin {
    result: String,
    repeated_attempts: usize,
    expected_clock_samples: usize,
    expected_target_observations: usize,
    expected_descriptor_adapter_calls: usize,
    expected_ledger_reservations: usize,
    expected_ledger_transitions: usize,
    expected_process_spawns: usize,
    expected_host_effects: usize,
    expected_plan_releases: usize,
    proof: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CrashCase {
    id: String,
    target: String,
    temp: String,
    expected: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FalsePassCase {
    acknowledgement: String,
    substitution: String,
    target: String,
    expected: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryContract {
    classification_is_read_only: bool,
    expected_prior_object_is_exact: bool,
    expected_next_object_is_exact: bool,
    expected_temporary_object_is_exact: bool,
    writable_expected_targets_are_rejected: bool,
    acknowledgement_is_bare_boolean: bool,
    acknowledgement_requires_canonical_bytes: bool,
    acknowledgement_binds_effect_identity: bool,
    acknowledgement_binds_publication_identity: bool,
    acknowledgement_binds_current_ledger_head: bool,
    acknowledgement_is_recovery_authorization: bool,
    unknown_missing_extra_or_ambiguous_fields_are_rejected: bool,
    separate_authorization_required: bool,
    authorization_binds_coordinator: bool,
    authorization_binds_current_ledger_head: bool,
    authorization_binds_ledger_generation: bool,
    authorization_binds_ledger_digest: bool,
    authorization_rejects_stale_or_replayed_head: bool,
    authorization_rejects_malformed_or_unknown_canonical_controls: bool,
    proposal_proof_binds_authorized_ledger_head: bool,
    authorization_expires: bool,
    proposal_performs_cleanup: bool,
    automatic_ambiguous_retry: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicApiContract {
    module_visibility: String,
    public_constructors: usize,
    cloneable_permit_or_handoff: bool,
    deserializable_permit_or_handoff: bool,
    production_descriptor_executor_implemented: bool,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn source(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).unwrap()
}

fn cases() -> Cases {
    serde_json::from_str(&source(
        "fixtures/supported-host-lifecycle-adapter/cases.json",
    ))
    .unwrap()
}

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

#[test]
fn darwin_contract_is_zero_reservation_zero_spawn_zero_effect_and_repeatable() {
    let darwin = cases().darwin;
    assert_eq!(darwin.result, "unsupported-platform");
    assert!(darwin.repeated_attempts >= 2);
    assert_eq!(darwin.expected_clock_samples, 0);
    assert_eq!(darwin.expected_target_observations, 0);
    assert_eq!(darwin.expected_descriptor_adapter_calls, 0);
    assert_eq!(darwin.expected_ledger_reservations, 0);
    assert_eq!(darwin.expected_ledger_transitions, 0);
    assert_eq!(darwin.expected_process_spawns, 0);
    assert_eq!(darwin.expected_host_effects, 0);
    assert_eq!(darwin.expected_plan_releases, 0);
    assert!(darwin.proof.contains("recursive"));

    let coordinator = source("validator/src/distribution/host_effect/lifecycle/coordinator.rs");
    let unsupported = coordinator
        .find("if platform == DescriptorExecutionPlatform::Darwin")
        .unwrap();
    for later in [
        "adapter.descriptor_capability()",
        "clock.sample()",
        "target.acquire(",
        ".ledger.head()",
        ".issue(binding)",
        ".reserve(reservation)",
        "custody.commit_release()",
    ] {
        assert!(unsupported < coordinator[unsupported..].find(later).unwrap() + unsupported);
    }
}

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

#[test]
fn lifecycle_candidate_has_no_public_construction_or_legacy_execution_route() {
    let cases = cases();
    assert_eq!(cases.public_api_contract.module_visibility, "crate-private");
    assert_eq!(cases.public_api_contract.public_constructors, 0);
    assert!(!cases.public_api_contract.cloneable_permit_or_handoff);
    assert!(!cases.public_api_contract.deserializable_permit_or_handoff);
    assert!(
        !cases
            .public_api_contract
            .production_descriptor_executor_implemented
    );

    let lifecycle = source("validator/src/distribution/host_effect/lifecycle.rs");
    assert!(!lifecycle.contains("pub mod "));
    for path in [
        "validator/src/distribution/host_effect/lifecycle.rs",
        "validator/src/distribution/host_effect/lifecycle/binding.rs",
        "validator/src/distribution/host_effect/lifecycle/coordinator.rs",
        "validator/src/distribution/host_effect/lifecycle/recovery.rs",
    ] {
        let body = source(path);
        assert!(
            !body.contains("pub fn new("),
            "public constructor in {path}"
        );
        assert!(
            !body.contains("fn into_parts("),
            "owned authority split in {path}"
        );
        for forbidden in [
            "HostAuthorization",
            "execute_authorized",
            "HostExecutor",
            "PreparedExternalHostEffect",
            ".into_plan(",
            "Command::new",
            "std::process",
        ] {
            assert!(
                !body.contains(forbidden),
                "legacy/effect route {forbidden} in {path}"
            );
        }
    }
    assert!(
        source("validator/src/distribution/host_effect/lifecycle/coordinator.rs")
            .contains("fn with_retained_authority")
    );
}
