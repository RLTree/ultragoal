use super::retired_behavior_routes::{Mapping, routed};

const BINDING: &[&str] = &[
    "local_issuer_binding_controls::closed_binding_refuses_loader_child_argv_program_and_policy_mutations",
];
const CONTEXT: &[&str] =
    &["local_issuer_binding_controls::context_mutation_refuses_before_authority_creation"];
const REPORT_ROWS: &[&str] = &[
    "report::mixed_witness_completion::unknown_duplicate_wrong_scope_and_invalid_failure_rows_are_rejected",
];
const REPORT_DEPENDENCY: &[&str] = &[
    "report::dependency_artifact_consistency::same_plan_reuse_and_execution_cannot_hide_missing_or_failed_dependencies",
];
const REPORT_COMPLETE: &[&str] = &[
    "report::mixed_witness_completion::complete_execution_requires_mixed_opaque_executed_and_verified_reuse_witnesses",
];
const NOOP: &[&str] = &["planning::clean_repository_is_an_exact_no_op"];

pub(crate) const MAP: &[Mapping] = &[
    routed(
        "runtime_adapter_cases/adapter_scenario.rs::fixture_catalog_is_exact_and_has_no_claim_effect",
        BINDING,
    ),
    routed(
        "runtime_adapter_cases/clean_noop.rs::clean_noop_is_deterministic_exact_and_non_effectful",
        NOOP,
    ),
    routed(
        "runtime_adapter_cases/clean_noop.rs::dirty_and_strict_preparation_bind_deterministic_complete_intents_without_effects",
        BINDING,
    ),
    routed(
        "runtime_adapter_cases/cross_request_rejection.rs::cross_request_witnesses_and_inconsistent_dependency_artifacts_fail_closed",
        REPORT_DEPENDENCY,
    ),
    routed(
        "runtime_adapter_cases/cross_request_rejection.rs::candidate_mutation_during_delegated_reconciliation_never_returns_complete",
        CONTEXT,
    ),
    routed(
        "runtime_adapter_cases/cross_request_rejection.rs::preparation_and_pre_report_refusal_source_has_no_effect_primitive",
        CONTEXT,
    ),
    routed(
        "runtime_adapter_cases/invocation_binding_refusals.rs::malformed_invocation_sets_and_stale_bindings_refuse_without_writes",
        BINDING,
    ),
    routed(
        "runtime_adapter_cases/loader_environment_rejection.rs::startup_loader_environment_variables_are_rejected_at_binding",
        BINDING,
    ),
    routed(
        "runtime_adapter_cases/loader_environment_rejection.rs::missing_runner_identity_and_unbounded_execution_policy_refuse_without_writes",
        BINDING,
    ),
    routed(
        "runtime_adapter_cases/outcome_set_rejection.rs::partial_duplicate_unknown_reordered_unobserved_and_cross_session_outcomes_fail_closed",
        REPORT_ROWS,
    ),
    routed(
        "runtime_adapter_cases/outcome_set_rejection.rs::genuine_ordered_outcomes_delegate_to_exact_report_reconciliation",
        REPORT_COMPLETE,
    ),
];
