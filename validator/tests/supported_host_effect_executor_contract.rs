use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

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
    fs::read_to_string(repo_root().join(path)).unwrap()
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

fn terminal_reobservation_guard_is_complete(source: &str) -> bool {
    let Some(start) = source.find("fn reobserve_terminal_failure") else {
        return false;
    };
    let Some(end_offset) = source[start..].find("fn finish_publication_failure") else {
        return false;
    };
    let window = &source[start..start + end_offset];
    let required_positions = [
        "head_before != head_after || record_before != record_after",
        "head_after != *record_after.current_head()",
        "record_after.reservation().permit_id() != effect.permit().permit_id()",
        "record_after == *effect.record()",
        "record_after.reservation() == effect.record().reservation()",
    ]
    .map(|token| window.find(token));
    let [
        Some(stability),
        Some(head_coherence),
        Some(permit_coherence),
        Some(still_in_flight),
        Some(terminal),
    ] = required_positions
    else {
        return false;
    };
    stability < head_coherence
        && head_coherence < still_in_flight
        && head_coherence < terminal
        && permit_coherence < still_in_flight
        && permit_coherence < terminal
}

fn post_reservation_reobservation_guard_is_complete(source: &str) -> bool {
    let Some(start) = source.find("fn reobserve_post_reservation") else {
        return false;
    };
    let rest = &source[start..];
    let end_offset = rest.find("#[cfg(test)]").unwrap_or(rest.len());
    let window = &rest[..end_offset];
    let required_positions = [
        "head_before != head_after || record_before != record_after",
        "head_after != *record_after.current_head()",
        "record_after.reservation().permit_id() != effect.permit().permit_id()",
        "classification: HostEffectPostReservationLedgerClassification::StillInFlight",
        "classification: HostEffectPostReservationLedgerClassification::TerminalObserved",
        "classification: HostEffectPostReservationLedgerClassification::ObservationRejected",
    ]
    .map(|token| window.find(token));
    let [
        Some(stability),
        Some(head_coherence),
        Some(permit_coherence),
        Some(still_in_flight),
        Some(terminal),
        Some(rejected),
    ] = required_positions
    else {
        return false;
    };
    stability < head_coherence
        && stability < permit_coherence
        && head_coherence < still_in_flight
        && head_coherence < terminal
        && head_coherence < rejected
        && permit_coherence < still_in_flight
        && permit_coherence < terminal
        && permit_coherence < rejected
}

#[test]
fn fixture_freezes_bounds_terminal_states_refusals_and_claim_ceiling() {
    let cases = cases();
    assert_eq!(cases.schema_version, "SupportedHostEffectExecutorCases-v1");
    assert_eq!(cases.activation.module_visibility, "crate-private");
    assert_eq!(cases.activation.public_constructor_count, 0);
    assert!(cases.activation.root_module_wiring_required);
    assert!(!cases.activation.public_cli_route_added);
    assert_eq!(
        cases.activation.darwin_external_process_execution,
        "unsupported-before-fork-spawn-write"
    );
    assert!(
        cases
            .activation
            .darwin_descriptor_relative_publication_tested_with_in_process_backend
    );
    assert_eq!(
        cases.activation.lifecycle_recovery_reexports_required,
        [
            "ExpectedPublicationObjectIdentity",
            "PublicationAcknowledgementIdentity",
            "PublicationExpectation",
        ]
    );
    assert_eq!(cases.supported_process_platforms.len(), 2);
    assert_eq!(
        cases
            .supported_process_platforms
            .iter()
            .map(|row| (row.platform.as_str(), row.primitive.as_str()))
            .collect::<Vec<_>>(),
        [("linux", "execveat-empty-path"), ("freebsd", "fexecve"),]
    );
    assert!(
        cases
            .supported_process_platforms
            .iter()
            .all(|row| !row.live_platform_tested)
    );
    assert_eq!(cases.transaction_order.len(), 12);

    let bounds = cases.execution_bounds;
    assert!(!bounds.environment_inherited);
    assert_eq!(bounds.accepted_environment_entries, 0);
    assert_eq!(bounds.stdout_limit_bytes, 1_048_576);
    assert_eq!(bounds.stderr_limit_bytes, 1_048_576);
    assert_eq!(bounds.maximum_timeout_ms, 300_000);
    assert!(!bounds.shell_execution);
    assert!(bounds.process_group_containment);
    assert_eq!(bounds.target_root_parent, "/private/tmp");
    assert_eq!(bounds.target_root_mode, "0700");
    assert_eq!(bounds.publication_mode, "0400");
    assert_eq!(bounds.publication_hard_links, 1);
    assert_eq!(bounds.publication_maximum_bytes, 1_048_576);

    let recovery = cases.terminal_transition_recovery;
    assert!(recovery.identity_bearing_handoff);
    assert!(recovery.effect_identity_bound);
    assert!(recovery.permit_id_bound);
    assert!(recovery.current_ledger_record_and_head_bound_when_observable);
    assert!(recovery.intended_outcome_bound);
    assert!(recovery.originating_error_bound);
    assert!(recovery.classification_bound);
    assert!(recovery.post_publication_prior_evidence_bound);
    assert!(recovery.post_publication_transition_cause_bound);
    assert!(recovery.post_publication_current_observation_unavailable_explicit);
    assert!(!recovery.post_publication_terminal_state_claim);
    assert!(!recovery.post_publication_acknowledgement_claim);
    assert!(!recovery.post_publication_success_claim);
    assert!(!recovery.post_publication_automatic_retry);
    assert!(!recovery.post_publication_automatic_cleanup);
    assert_eq!(recovery.stable_observation_attempts, 3);
    assert!(recovery.global_head_record_coherence_required);
    assert!(recovery.record_permit_coherence_required);
    assert!(!recovery.mismatched_head_record_exact_classification);
    assert_eq!(
        recovery.mismatched_head_record_fallback,
        "ledger-observation-unavailable"
    );
    assert_eq!(
        recovery
            .classifications
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "still-in-flight",
            "terminal-committed-and-verified",
            "terminal-committed-but-unverifiable",
            "ledger-observation-rejected",
            "ledger-observation-unavailable",
        ]
    );
    assert_eq!(
        recovery
            .post_publication_classifications
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["committed-before-terminal-transition-observation-unavailable"]
    );
    assert_eq!(
        recovery
            .post_reservation_ledger_classifications
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "still-in-flight",
            "terminal-observed",
            "observation-rejected",
            "observation-unavailable",
        ]
    );
    assert_eq!(
        recovery
            .post_reservation_publication_classifications
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "no-publication-evidence",
            "publication-evidence-unavailable",
            "publication-identity-only-current-observation-unavailable",
            "prior-observation-current-observation-unavailable",
        ]
    );
    assert!(!recovery.bare_ledger_substitution_after_opaque_handoff);
    assert!(!recovery.bare_post_publication_reobservation_failure_after_opaque_handoff);
    assert!(recovery.guarded_post_reservation_error_boundary);
    assert_eq!(recovery.identity_free_returns_after_guard, 0);
    assert_eq!(recovery.terminal_only_returns_after_guard, 0);
    assert!(recovery.complete_error_chain_bound);
    assert!(recovery.ordered_error_chain_bound);
    assert!(recovery.terminal_transition_error_bound);
    assert!(recovery.duplicate_error_ids_rejected);
    assert!(recovery.synthetic_recovery_error_id_rejected);
    assert_eq!(recovery.maximum_originating_error_chain_length, 4);

    assert_eq!(cases.terminal_state_matrix.len(), 23);
    let terminals = cases
        .terminal_state_matrix
        .iter()
        .map(|row| {
            (
                row.case.as_str(),
                row.command_started,
                row.publication_may_exist,
                row.state.as_str(),
                row.recovery_required,
            )
        })
        .collect::<Vec<_>>();
    assert!(terminals.contains(&(
        "started-backend-failure-terminal-transition-failed-before-mutation",
        true,
        false,
        "in-flight",
        true,
    )));
    assert!(terminals.contains(&(
        "publication-failure-terminal-commit-target-reobservation-unavailable",
        true,
        true,
        "ambiguous",
        true,
    )));
    assert!(terminals.contains(&(
        "committed-publication-terminal-commit-target-reobservation-unavailable",
        true,
        true,
        "settled",
        true,
    )));
    assert!(terminals.contains(&(
        "publication-failure-terminal-io-target-reobservation-unavailable",
        true,
        true,
        "in-flight",
        true,
    )));
    assert!(terminals.contains(&(
        "started-backend-failure-terminal-transition-committed-but-unverifiable",
        true,
        false,
        "ambiguous",
        true,
    )));
    assert!(terminals.contains(&(
        "started-backend-failure-terminal-record-mutation-rejected",
        true,
        false,
        "terminal-claim-withheld",
        true,
    )));
    assert!(terminals.contains(&(
        "failure-during-publication-terminal-ledger-transition-failed",
        true,
        true,
        "in-flight",
        true,
    )));
    assert!(terminals.contains(&(
        "publication-committed-terminal-ledger-transition-failed",
        true,
        true,
        "in-flight",
        true,
    )));
    assert!(terminals.contains(&(
        "publication-committed-terminal-transition-and-target-reobservation-failed",
        true,
        true,
        "terminal-claim-withheld",
        true,
    )));
    assert!(terminals.contains(&(
        "acknowledgement-invalid-after-settlement",
        true,
        true,
        "settled",
        true,
    )));
    for case in [
        "unrelated-permit-global-head-advance-with-still-in-flight-record",
        "repeated-unrelated-head-advances-after-terminal-commit",
        "unrelated-head-advance-record-substitution",
        "head-rollback-after-terminal-and-unrelated-advance",
        "observation-error-after-unrelated-head-advance",
        "post-reservation-wrong-permit-in-flight-observation",
        "post-reservation-wrong-permit-terminal-observation",
        "post-reservation-wrong-permit-rejected-observation",
        "post-reservation-right-record-wrong-head-observation",
    ] {
        assert!(terminals.contains(&(case, true, false, "terminal-claim-withheld", true)));
    }

    let refusals = cases
        .required_refusal_cases
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(refusals.len(), cases.required_refusal_cases.len());
    for required in [
        "replay-after-terminal-record",
        "ledger-rollback-or-stale-head",
        "target-path-swap",
        "executable-mutation-between-commands",
        "nonempty-environment",
        "stdout-overflow",
        "timeout-after-process-start",
        "symlink-object",
        "hardlinked-object",
        "rename-race",
        "directory-fsync-failure",
        "prepublication-terminal-transition-failure-before-mutation",
        "prepublication-terminal-transition-committed-but-unverifiable",
        "terminal-recovery-binding-mutation",
        "terminal-record-state-mutation",
        "terminal-ledger-transition-failure-after-publication",
        "terminal-ledger-transition-and-target-reobservation-failure-after-publication",
        "publication-failure-terminal-commit-target-reobservation-loss",
        "committed-publication-terminal-commit-target-reobservation-loss",
        "publication-failure-terminal-transition-io-target-reobservation-loss",
        "unrelated-permit-global-head-advance",
        "repeated-global-head-advance",
        "recovery-record-substitution",
        "recovery-head-rollback-staleness",
        "recovery-observation-error-after-head-advance",
        "post-reservation-wrong-permit-record",
        "post-reservation-right-record-wrong-head",
        "partial-acknowledgement",
        "false-pass-acknowledgement",
        "darwin-native-process-backend",
    ] {
        assert!(refusals.contains(required), "missing refusal {required}");
    }

    let source_contract = cases.production_source_contract;
    assert!(source_contract.retained_executable_descriptor_only);
    assert!(source_contract.exact_accepted_argv_only);
    assert!(!source_contract.path_lookup_or_shell_fallback);
    assert!(source_contract.descriptor_relative_target_operations);
    assert!(source_contract.no_follow_and_exclusive_creation);
    assert!(source_contract.rename_no_replace);
    assert!(source_contract.file_and_directory_fsync);
    assert!(source_contract.publication_before_terminal_ledger_transition);
    assert!(source_contract.terminal_ledger_transition_before_acknowledgement);
    assert!(!source_contract.bare_boolean_acknowledgement);
    assert!(!source_contract.automatic_ambiguous_retry);

    let ceiling = cases.claim_ceiling;
    assert!(ceiling.source_candidate);
    assert!(ceiling.macos_descriptor_relative_publication_unit_proof);
    assert!(!ceiling.macos_external_process_execution);
    assert!(!ceiling.linux_external_process_runtime_proof);
    assert!(!ceiling.freebsd_external_process_runtime_proof);
    assert!(!ceiling.public_route);
    assert!(!ceiling.installed_runtime);
    assert!(!ceiling.representative_real_host_journey);
    assert!(!ceiling.acceptance_or_release);
}

#[test]
fn executor_orders_command_publication_ledger_and_acknowledgement() {
    let executor = source("validator/src/distribution/host_effect/executor.rs");
    for required in [
        "DescriptorExecutionHandoff",
        "with_retained_authority",
        "has_current_in_flight_record",
        "self.backend.execute",
        ".prepare(&effect_identity_sha256",
        ".publish(prepared",
        "HostEffectState::Settled",
        "PublicationAcknowledgementIdentity::new",
        "from_canonical_json",
        ".acknowledge(",
        "PublicationClassificationId::AcknowledgedCommitted",
        "committed_before_terminal_recovery_failure",
        "prepublication_terminal_recovery_failure",
        "reobserve_terminal_failure",
        "HostEffectTerminalRecoveryClassification::StillInFlight",
        "HostEffectTerminalRecoveryClassification::TerminalCommittedAndVerified",
        "HostEffectTerminalRecoveryClassification::TerminalCommittedButUnverifiable",
        "post_reservation_recovery_failure",
    ] {
        assert!(
            executor.contains(required),
            "missing executor token {required}"
        );
    }
    assert_before(
        &executor,
        "self.backend.execute",
        ".prepare(&effect_identity_sha256",
    );
    assert_before(&executor, ".publish(prepared", "let terminal = match");
    assert_before(
        &executor,
        "let terminal = match",
        "PublicationAcknowledgementIdentity::new",
    );
    assert_before(
        &executor,
        "PublicationAcknowledgementIdentity::new",
        ".acknowledge(",
    );
    assert!(!executor.contains("pub fn "));
    assert!(!executor.contains("std::process::Command"));
    assert!(!executor.contains("self.transition_terminal(effect, state, outcome_sha256)?;"));

    let model = source("validator/src/distribution/host_effect/executor/model.rs");
    for required in [
        "HostEffectRecoveryHandoff::TerminalTransition",
        "effect_identity_sha256",
        "permit_id",
        "ledger_head",
        "ledger_record",
        "exact_current_ledger_observation",
        "outcome",
        "originating_error_ids",
        "classification",
        "binding_sha256",
        "verify_binding",
        "HostEffectRecoveryHandoff::PostPublicationTerminalTransition",
        "HostEffectRecoveryHandoff::PostReservation",
        "prior_publication_observation",
        "exact_current_publication_observation",
        "CommittedBeforeTerminalTransitionObservationUnavailable",
        "HostEffectPostReservationLedgerClassification",
        "HostEffectPostReservationPublicationClassification",
        "originating_error_ids",
    ] {
        assert!(
            model.contains(required),
            "missing recovery model token {required}"
        );
    }

    let tests = source("validator/src/distribution/host_effect/executor/tests.rs");
    for required in [
        "positive_actual_publication_fsync_rename_ledger_and_ack_transaction_settles",
        "negative_started_backend_terminal_failure_before_mutation_returns_exact_recovery",
        "negative_clock_failure_terminal_transition_also_returns_exact_recovery",
        "negative_preflight_ledger_substitution_after_handoff_is_identity_bearing",
        "race_committed_terminal_error_is_reobserved_as_committed_but_unverifiable",
        "security_mutated_terminal_record_is_rejected_without_false_terminal_claim",
        "mutation_of_recovery_permit_binding_is_detected_as_false_pass",
        "mutation_of_recovery_error_chain_order_length_and_members_is_detected",
        "dual_failure_after_committed_publication_retains_identity_when_reobservation_is_unavailable",
        "publication_failure_terminal_commit_then_reobservation_loss_retains_identity",
        "committed_publication_terminal_commit_then_reobservation_loss_retains_identity",
        "publication_failure_terminal_io_then_reobservation_loss_retains_identity",
        "race_unrelated_permit_global_head_advance_with_still_in_flight_record_is_unavailable_and_bound",
        "race_repeated_unrelated_head_advances_after_terminal_commit_withhold_terminal_classification",
        "negative_unrelated_head_advance_record_substitution_is_unavailable_and_bound",
        "negative_head_rollback_after_terminal_and_unrelated_advance_is_unavailable_and_bound",
        "negative_observation_error_after_unrelated_head_advance_is_unavailable_and_bound",
        "wrong_permit_post_reservation_still_in_flight_observation_falls_back_bound_unavailable",
        "wrong_permit_post_reservation_terminal_observation_falls_back_bound_unavailable",
        "wrong_permit_post_reservation_rejected_observation_falls_back_bound_unavailable",
        "post_reservation_right_record_wrong_head_falls_back_bound_unavailable",
        "post_reservation_correct_permit_still_in_flight_positive_control_is_exact_and_bound",
        "post_reservation_correct_permit_terminal_positive_control_is_exact_and_bound",
        "post_reservation_correct_permit_rejected_positive_control_is_exact_and_bound",
        "post_reservation_observation_error_falls_back_bound_unavailable_and_preserves_chain",
        "wrong_permit_post_reservation_replay_preflight_route_falls_back_bound_unavailable",
        "FailTerminalLedger",
        "CommitThenFailTerminalLedger",
        "DisplaceTargetAndFailTerminalLedger",
        "CommitThenDisplaceTargetLedger",
        "AdvanceUnrelatedPermitThenFailTerminalLedger",
    ] {
        assert!(
            tests.contains(required),
            "missing named recovery test {required}"
        );
    }

    let dual_failure_start = executor
        .find("fn committed_before_terminal_recovery_failure")
        .unwrap();
    let dual_failure_end = executor[dual_failure_start..]
        .find("fn publication_before_terminal_recovery_failure")
        .map(|offset| dual_failure_start + offset)
        .unwrap();
    let dual_failure = &executor[dual_failure_start..dual_failure_end];
    assert!(dual_failure.contains("post_reservation_recovery_failure"));
    assert!(dual_failure.contains("observation: Some(&committed.observation)"));
    assert!(dual_failure.contains("originating_error_id"));
    assert!(dual_failure.contains("observation_failure.id()"));
    assert!(!dual_failure.contains("HostEffectExecutorFailure::new"));
    assert!(!dual_failure.contains("HostEffectExecutorFailure::terminal"));
}

#[test]
fn terminal_reobservation_requires_coherent_global_head_before_exact_classification() {
    let executor = source("validator/src/distribution/host_effect/executor.rs");
    assert!(terminal_reobservation_guard_is_complete(&executor));

    for (original, replacement) in [
        ("head_after != *record_after.current_head()", "false"),
        (
            "head_after != *record_after.current_head()",
            "head_before != *record_after.current_head()",
        ),
        (
            "head_after != *record_after.current_head()",
            "head_after != *record_before.current_head()",
        ),
        (
            "record_after.reservation().permit_id() != effect.permit().permit_id()",
            "false",
        ),
    ] {
        let mutated = executor.replacen(original, replacement, 1);
        assert!(
            !terminal_reobservation_guard_is_complete(&mutated),
            "guard mutation was accepted: {replacement}"
        );
    }
}

#[test]
fn post_reservation_reobservation_requires_coherent_head_and_recovering_permit_before_exact_classification()
 {
    let executor = source("validator/src/distribution/host_effect/executor.rs");
    let post_reservation =
        source_window(&executor, "fn reobserve_post_reservation", "#[cfg(test)]");
    assert!(post_reservation_reobservation_guard_is_complete(
        &post_reservation
    ));

    for (original, replacement) in [
        ("head_after != *record_after.current_head()", "false"),
        (
            "head_after != *record_after.current_head()",
            "head_before != *record_after.current_head()",
        ),
        (
            "head_after != *record_after.current_head()",
            "head_after != *record_before.current_head()",
        ),
        (
            "record_after.reservation().permit_id() != effect.permit().permit_id()",
            "false",
        ),
        (
            "record_after.reservation().permit_id() != effect.permit().permit_id()",
            "record_before.reservation().permit_id() != effect.permit().permit_id()",
        ),
        (
            "record_after.reservation().permit_id() != effect.permit().permit_id()",
            "effect.record().reservation().permit_id() != effect.permit().permit_id()",
        ),
    ] {
        let mutated = post_reservation.replacen(original, replacement, 1);
        assert!(
            !post_reservation_reobservation_guard_is_complete(&mutated),
            "post-reservation guard mutation was accepted: {replacement}"
        );
    }

    let permit_guard = "record_after.reservation().permit_id() != effect.permit().permit_id()";
    let without_guard = post_reservation.replacen(permit_guard, "false", 1);
    let moved_after_exact = without_guard.replacen(
        "classification: HostEffectPostReservationLedgerClassification::StillInFlight,",
        &format!(
            "classification: HostEffectPostReservationLedgerClassification::StillInFlight,\n                 // Mutant: guard moved after exact classification.\n                 if {permit_guard} {{ continue; }}"
        ),
        1,
    );
    assert!(
        !post_reservation_reobservation_guard_is_complete(&moved_after_exact),
        "a permit guard moved after exact StillInFlight classification was accepted"
    );
}

#[test]
fn post_reservation_boundary_enumerates_and_rejects_identity_free_returns() {
    let executor = source("validator/src/distribution/host_effect/executor.rs");
    let handoff = source_window(&executor, "fn execute_handoff", "fn execute_authorized");
    let guard = source_window(&executor, "fn execute_authorized", "fn execute_reserved");
    let publication_terminal = source_window(
        &executor,
        "fn finish_publication_failure",
        "fn committed_recovery_failure",
    );
    let backend_terminal = source_window(
        &executor,
        "fn finish_backend_failure",
        "fn finish_started_failure",
    );
    let timestamp_terminal = source_window(
        &executor,
        "fn finish_terminal_with_timestamp",
        "fn verified_terminal_failure",
    );
    let prepublication_terminal = source_window(
        &executor,
        "fn prepublication_terminal_recovery_failure",
        "fn identity_bearing_preflight_ledger_failure",
    );
    let committed_terminal = source_window(
        &executor,
        "fn committed_recovery_failure",
        "fn committed_before_terminal_recovery_failure",
    );
    let publication_transition = source_window(
        &executor,
        "fn publication_before_terminal_recovery_failure",
        "fn post_reservation_recovery_failure",
    );
    let test_entry = source_window(
        &executor,
        "fn execute_authorized_for_test",
        "fn receipt_name",
    );

    assert!(handoff.contains("post_reservation_recovery_failure"));
    assert!(guard.contains("match self.execute_reserved"));
    assert!(guard.contains("failure.recovery().is_none()"));
    assert!(guard.contains("post_reservation_recovery_failure"));
    assert_eq!(executor.matches("self.execute_reserved(").count(), 1);
    assert!(test_entry.contains("self.execute_authorized("));
    assert!(!test_entry.contains("self.execute_reserved("));

    let guarded_surfaces = [
        ("opaque-handoff-entry", handoff),
        ("post-reservation-result-guard", guard),
        ("backend-prepublication-terminal", backend_terminal),
        (
            "clock-and-target-prepublication-terminal",
            timestamp_terminal,
        ),
        ("prepublication-terminal-recovery", prepublication_terminal),
        (
            "publication-terminal-reobservation-loss",
            publication_terminal,
        ),
        ("committed-terminal-reobservation-loss", committed_terminal),
        (
            "publication-transition-reobservation-loss",
            publication_transition,
        ),
        ("unit-test-entry", test_entry),
    ];
    for (surface, source) in guarded_surfaces {
        for forbidden in [
            "HostEffectExecutorFailure::terminal",
            "return Err(HostEffectExecutorFailure::new",
            "Err(_) => HostEffectExecutorFailure::new",
            "Err(_) =>",
        ] {
            assert!(
                !source.contains(forbidden),
                "identity-free post-reservation return on {surface}: {forbidden}"
            );
        }
    }
    for specialized in [
        publication_terminal,
        committed_terminal,
        publication_transition,
    ] {
        assert!(specialized.contains("post_reservation_recovery_failure"));
        assert!(specialized.contains("observation_failure.id()"));
    }
    assert!(!executor.contains("HostEffectExecutorFailure::terminal"));

    for prepublication in [backend_terminal, timestamp_terminal] {
        assert!(prepublication.contains("Err(transition_failure)"));
        assert!(prepublication.contains("append_error_cause"));
        assert!(prepublication.contains("transition_failure.id()"));
        assert!(prepublication.contains("originating_error_ids"));
    }

    let model = source("validator/src/distribution/host_effect/executor/model.rs");
    assert!(model.contains("HostEffectRecoveryHandoff::PostReservation"));
    assert!(model.contains("host-effect-post-reservation-recovery.v1"));
    assert!(model.contains("originating_error_ids"));
    assert!(model.contains("host-effect-terminal-recovery.v2"));
    assert!(model.contains("valid_originating_error_chain"));
    assert!(model.contains("MAX_ORIGINATING_ERROR_CHAIN_LENGTH: usize = 4"));
    assert!(model.contains("HostEffectExecutorErrorId::RecoveryRequired"));
    assert!(model.contains("exact_current_ledger_observation"));
    assert!(model.contains("exact_current_publication_observation"));
    assert!(!model.contains("pub(super) const fn terminal("));
}

#[test]
fn process_backend_is_descriptor_bound_empty_environment_contained_and_darwin_closed() {
    let process = source("validator/src/distribution/host_effect/executor/process.rs");
    let model = source("validator/src/distribution/host_effect/executor/model.rs");
    for required in [
        "execveat(",
        "libc::AT_EMPTY_PATH",
        "fexecve(",
        "executable.file().as_raw_fd()",
        "let environment = [std::ptr::null",
        "libc::setpgid",
        "terminate_process_group",
        "libc::kill(-child",
        "policy.stdout_limit()",
        "policy.stderr_limit()",
        "policy.timeout()",
        "UnsupportedPlatform",
        "defense in depth and performs no fork, spawn, or write",
    ] {
        assert!(
            process.contains(required),
            "missing process token {required}"
        );
    }
    assert!(!process.contains("Command::new"));
    assert!(!process.contains("/bin/sh"));
    assert!(!process.contains("/usr/bin/env"));
    for required in [
        "EXACT_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024",
        "MAX_TIMEOUT_MS: u64 = 5 * 60 * 1000",
        "if !environment.is_empty()",
        "EnvironmentInjection",
        "inherited_environment: false",
        "environment_entries: 0",
    ] {
        assert!(model.contains(required), "missing model token {required}");
    }
}

#[test]
fn target_publication_is_descriptor_relative_exclusive_synced_and_exactly_reobserved() {
    let target = source("validator/src/distribution/host_effect/executor/target.rs");
    let normalized = target.split_whitespace().collect::<Vec<_>>().join(" ");
    for required in [
        "libc::openat(",
        "libc::O_NOFOLLOW",
        "libc::O_EXCL",
        "file.write_all(&prepared.bytes)",
        "libc::fchmod",
        "file.sync_all()",
        "rename_noreplace(",
        "libc::renameatx_np(",
        "libc::RENAME_EXCL",
        "RENAME_NOFOLLOW_ANY",
        "RENAME_RESOLVE_BENEATH",
        "BeforeDirectoryFsync",
        "CommittedBeforeAcknowledgement",
        "initially_created.links != 1",
        "before.links != 1",
        "PublicationObjectKind::Symlink",
        "PublicationObjectKind::Fifo",
        "PublicationObjectKind::Socket",
        "PublicationObjectKind::Device",
    ] {
        assert!(target.contains(required), "missing target token {required}");
    }
    assert_before(
        &target,
        "file.write_all(&prepared.bytes)",
        "file.sync_all()",
    );
    assert_before(&target, "file.sync_all()", "rename_noreplace(");
    assert!(normalized.contains("self.anchor .directory .sync_all()"));
    assert_before(
        &normalized,
        "rename_noreplace(",
        "self.anchor .directory .sync_all()",
    );
    assert!(!target.contains("fs::rename("));
    assert!(!target.contains("File::create("));
}
