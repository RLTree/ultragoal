use super::invariant_control_map::{
    CHILD_LIFECYCLE_BLOCKER, CHILD_SUCCESS_BLOCKER, Mapping, blocked, routed,
};

const BROKER: &[&str] =
    &["current_path_controls::root_broker_gate_refuses_before_spawn_and_writes"];
const BINDING: &[&str] = &[
    "current_path_controls::closed_binding_refuses_loader_child_argv_program_and_policy_mutations",
];
const CONTEXT: &[&str] =
    &["current_path_controls::context_mutation_refuses_before_authority_creation"];
const OUTPUTS: &[&str] = &[
    "filesystem_controls::output_symlink_hardlink_fifo_socket_and_stale_files_refuse_exact_capture",
    "filesystem_controls::output_nested_swap_and_create_delete_restore_refuse_final_validation",
];
const RECOVERY: &[&str] = &[
    "routine_work::runtime_adapter::production::ledger::tests::staged_publication_recovers_without_reauthorizing_artifact_bytes",
];
const CONFLICT: &[&str] = &[
    "routine_work::runtime_adapter::production::ledger::tests::expired_and_concurrently_conflicting_preparations_fail_closed",
];
const REUSE: &[&str] = &[
    "routine_work::runtime_adapter::production::ledger::tests::candidate_source_command_and_cross_attempt_substitution_never_prepare",
    "reuse::matching::content_and_same_size_output_substitution_never_hit",
];
const PLAN: &[&str] = &["planning::strict_named_boundary_and_optional_fallback_are_explicit"];
const IMMUTABLE: &[&str] = &[
    "routine_work::runtime_adapter::mediator::filesystem::mediation_failure::tests::current_user_owned_0555_executable_is_mutable_despite_denied_write_access",
];
const SYSTEM: &[&str] = &[
    "routine_work::runtime_adapter::mediator::filesystem::mediation_failure::tests::root_owned_system_shell_path_remains_eligible_for_non_root_effective_user",
];

pub(crate) const MAP: &[Mapping] = &[
    routed(
        "runtime_mediator_cases/reuse_integrity.rs::fabricated_or_mutated_reuse_never_becomes_a_cache_hit",
        REUSE,
    ),
    routed(
        "runtime_mediator_cases/reuse_integrity.rs::strict_expansion_and_capability_fallback_execute_the_selected_plan_exactly",
        PLAN,
    ),
    routed(
        "runtime_mediator_cases/reuse_integrity.rs::current_user_owned_0555_runner_is_rejected_before_spawn_or_effect",
        IMMUTABLE,
    ),
    routed(
        "runtime_mediator_cases/script_replacement_rejection.rs::external_dash_script_replacement_cannot_execute_or_seed_reuse",
        BINDING,
    ),
    blocked(
        "runtime_mediator_cases/script_replacement_rejection.rs::bound_dash_source_executes_reuses_exactly_and_mutation_invalidates_reuse",
        CHILD_SUCCESS_BLOCKER,
        BINDING,
    ),
    blocked(
        "runtime_mediator_cases/setup_failure_recovery.rs::post_spawn_setup_failures_cleanup_every_resource_and_require_exact_recovery",
        CHILD_LIFECYCLE_BLOCKER,
        RECOVERY,
    ),
    routed(
        "runtime_mediator_cases/setup_failure_recovery.rs::missing_forged_and_replayed_root_authority_fail_closed",
        BROKER,
    ),
    routed(
        "runtime_mediator_cases/system_shell_eligibility.rs::root_owned_system_shell_remains_eligible_for_non_root_execution",
        SYSTEM,
    ),
    routed(
        "runtime_mediator_cases/system_shell_eligibility.rs::concurrent_duplicate_grant_and_request_have_exactly_one_winner",
        CONFLICT,
    ),
    routed(
        "runtime_mediator_cases/system_shell_eligibility.rs::active_protocol_attempt_blocks_a_distinct_grant_before_second_spawn",
        CONFLICT,
    ),
    routed(
        "runtime_mediator_cases/system_shell_eligibility.rs::stale_dirty_bytes_and_output_scope_swap_are_refused_before_spawn",
        CONTEXT,
    ),
    routed(
        "runtime_mediator_cases/unsafe_output_rejection.rs::unsafe_output_objects_are_rejected_before_authority_is_consumed",
        OUTPUTS,
    ),
    routed(
        "runtime_mediator_cases/unsafe_output_rejection.rs::descriptor_walk_refuses_a_nested_directory_swap_during_capture",
        OUTPUTS,
    ),
    blocked(
        "runtime_mediator_cases/unsafe_output_rejection.rs::cleared_environment_and_process_exit_are_authoritative",
        CHILD_SUCCESS_BLOCKER,
        BINDING,
    ),
    blocked(
        "runtime_mediator_cases/unsafe_output_rejection.rs::sandbox_denies_undeclared_writes_and_network_connections",
        CHILD_LIFECYCLE_BLOCKER,
        BROKER,
    ),
];
