#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cases {
    schema_version: String,
    activation: Activation,
    supported_process_platforms: Vec<ProcessPlatform>,
    transaction_order: Vec<String>,
    execution_bounds: ExecutionBounds,
    terminal_transition_recovery: TerminalTransitionRecovery,
    terminal_state_matrix: Vec<TerminalCase>,
    required_refusal_cases: Vec<String>,
    production_source_contract: ProductionSourceContract,
    claim_ceiling: ClaimCeiling,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Activation {
    module_visibility: String,
    public_constructor_count: usize,
    root_module_wiring_required: bool,
    lifecycle_recovery_reexports_required: Vec<String>,
    public_cli_route_added: bool,
    darwin_external_process_execution: String,
    darwin_descriptor_relative_publication_tested_with_in_process_backend: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessPlatform {
    platform: String,
    primitive: String,
    live_platform_tested: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionBounds {
    environment_inherited: bool,
    accepted_environment_entries: usize,
    stdout_limit_bytes: usize,
    stderr_limit_bytes: usize,
    maximum_timeout_ms: u64,
    shell_execution: bool,
    process_group_containment: bool,
    target_root_parent: String,
    target_root_mode: String,
    publication_mode: String,
    publication_hard_links: u64,
    publication_maximum_bytes: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TerminalTransitionRecovery {
    identity_bearing_handoff: bool,
    effect_identity_bound: bool,
    permit_id_bound: bool,
    current_ledger_record_and_head_bound_when_observable: bool,
    intended_outcome_bound: bool,
    originating_error_bound: bool,
    classification_bound: bool,
    post_publication_prior_evidence_bound: bool,
    post_publication_transition_cause_bound: bool,
    post_publication_current_observation_unavailable_explicit: bool,
    post_publication_terminal_state_claim: bool,
    post_publication_acknowledgement_claim: bool,
    post_publication_success_claim: bool,
    post_publication_automatic_retry: bool,
    post_publication_automatic_cleanup: bool,
    stable_observation_attempts: usize,
    global_head_record_coherence_required: bool,
    record_permit_coherence_required: bool,
    mismatched_head_record_exact_classification: bool,
    mismatched_head_record_fallback: String,
    classifications: Vec<String>,
    post_publication_classifications: Vec<String>,
    post_reservation_ledger_classifications: Vec<String>,
    post_reservation_publication_classifications: Vec<String>,
    bare_ledger_substitution_after_opaque_handoff: bool,
    bare_post_publication_reobservation_failure_after_opaque_handoff: bool,
    guarded_post_reservation_error_boundary: bool,
    identity_free_returns_after_guard: usize,
    terminal_only_returns_after_guard: usize,
    complete_error_chain_bound: bool,
    ordered_error_chain_bound: bool,
    terminal_transition_error_bound: bool,
    duplicate_error_ids_rejected: bool,
    synthetic_recovery_error_id_rejected: bool,
    maximum_originating_error_chain_length: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TerminalCase {
    case: String,
    command_started: bool,
    publication_may_exist: bool,
    state: String,
    recovery_required: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionSourceContract {
    retained_executable_descriptor_only: bool,
    exact_accepted_argv_only: bool,
    path_lookup_or_shell_fallback: bool,
    descriptor_relative_target_operations: bool,
    no_follow_and_exclusive_creation: bool,
    rename_no_replace: bool,
    file_and_directory_fsync: bool,
    publication_before_terminal_ledger_transition: bool,
    terminal_ledger_transition_before_acknowledgement: bool,
    bare_boolean_acknowledgement: bool,
    automatic_ambiguous_retry: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimCeiling {
    source_candidate: bool,
    macos_descriptor_relative_publication_unit_proof: bool,
    macos_external_process_execution: bool,
    linux_external_process_runtime_proof: bool,
    freebsd_external_process_runtime_proof: bool,
    public_route: bool,
    installed_runtime: bool,
    representative_real_host_journey: bool,
    acceptance_or_release: bool,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn source(path: &str) -> String {
    let requested = repo_root().join(path);
    if requested.is_file() {
        return fs::read_to_string(requested).unwrap();
    }
    let module_root = requested.with_extension("").join("mod.rs");
    let mut visited = BTreeSet::new();
    expand_module_source(&module_root, &mut visited, 0)
}

fn expand_module_source(path: &Path, visited: &mut BTreeSet<PathBuf>, depth: usize) -> String {
    assert!(depth <= 32, "module source expansion exceeded depth bound");
    let canonical = fs::canonicalize(path).unwrap();
    assert!(
        canonical.starts_with(repo_root()),
        "module source escaped repository"
    );
    assert!(visited.insert(canonical.clone()), "cyclic module include");
    let text = fs::read_to_string(&canonical).unwrap();
    assert!(text.len() <= 4 * 1024 * 1024, "module source too large");
    let mut expanded = String::new();
    for line in text.lines() {
        if let Some(relative) = include_path(line) {
            expanded.push_str(&expand_module_source(
                &canonical.parent().unwrap().join(relative),
                visited,
                depth + 1,
            ));
        } else {
            expanded.push_str(line);
            expanded.push('\n');
        }
    }
    visited.remove(&canonical);
    expanded
}

fn include_path(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix("include!(\"")?
        .strip_suffix("\");")
}

fn cases() -> Cases {
    serde_json::from_str(&source(
        "fixtures/supported-host-effect-executor/cases.json",
    ))
    .unwrap()
}

fn assert_before(source: &str, prior: &str, next: &str) {
    let prior = source
        .find(prior)
        .unwrap_or_else(|| panic!("missing {prior}"));
    let next = source
        .find(next)
        .unwrap_or_else(|| panic!("missing {next}"));
    assert!(prior < next, "{prior} must precede {next}");
}

fn source_window<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start = source
        .find(start)
        .unwrap_or_else(|| panic!("missing start token {start}"));
    let end = source[start..]
        .find(end)
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("missing end token {end}"));
    &source[start..end]
}
