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
