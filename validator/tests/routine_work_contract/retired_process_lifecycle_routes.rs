//! Retired process-lifecycle invariants and their bounded current routes.

use super::retired_behavior_routes::{
    CHILD_LIFECYCLE_BLOCKER, CHILD_SUCCESS_BLOCKER, Mapping, blocked, routed,
};

const BROKER: &[&str] =
    &["broker_binding_controls::root_broker_gate_refuses_before_spawn_and_writes"];
const BINDING: &[&str] = &[
    "broker_binding_controls::closed_binding_refuses_loader_child_argv_program_and_policy_mutations",
];
const CONTEXT: &[&str] =
    &["broker_binding_controls::context_mutation_refuses_before_authority_creation"];
const READS: &[&str] =
    &["filesystem_controls::read_symlink_hardlink_fifo_socket_ancestor_swap_and_restore_refuse"];
const IMMUTABLE: &[&str] = &[
    "routine_work::runtime_adapter::mediator::filesystem::mediation_failure::tests::current_user_owned_0555_executable_is_mutable_despite_denied_write_access",
];
const NOOP: &[&str] = &[
    "authority_ledger_controls::exact_noop_bypasses_authority_initialization_and_workspace_writes",
];

pub(crate) const MAP: &[Mapping] = &[
    blocked(
        "runtime_mediator_cases/cancellation_reaping.rs::cancellation_kills_the_process_group_and_emits_no_reuse",
        CHILD_LIFECYCLE_BLOCKER,
        BROKER,
    ),
    blocked(
        "runtime_mediator_cases/cancellation_reaping.rs::timeout_requires_exact_recovery_authority_and_leaves_no_child",
        CHILD_LIFECYCLE_BLOCKER,
        BROKER,
    ),
    blocked(
        "runtime_mediator_cases/context_mutation_recovery.rs::post_spawn_context_mutation_requires_exact_recovery_before_retry",
        CHILD_LIFECYCLE_BLOCKER,
        CONTEXT,
    ),
    blocked(
        "runtime_mediator_cases/context_mutation_recovery.rs::post_spawn_finish_failure_keeps_the_exact_recovery_barrier",
        CHILD_LIFECYCLE_BLOCKER,
        CONTEXT,
    ),
    blocked(
        "runtime_mediator_cases/executable_substitution_rejection.rs::compiled_same_process_executable_substitutions_are_denied_without_success_or_reuse",
        CHILD_LIFECYCLE_BLOCKER,
        BINDING,
    ),
    blocked(
        "runtime_mediator_cases/executable_substitution_rejection.rs::exact_same_executable_reexec_remains_single_process_and_can_complete",
        CHILD_SUCCESS_BLOCKER,
        BROKER,
    ),
    blocked(
        "runtime_mediator_cases/executable_substitution_rejection.rs::compiled_user_owned_dlopen_mapping_is_denied_without_effect_success_or_reuse",
        CHILD_LIFECYCLE_BLOCKER,
        IMMUTABLE,
    ),
    blocked(
        "runtime_mediator_cases/executable_substitution_rejection.rs::pinned_single_process_route_completes_and_is_absent_after_natural_exit",
        CHILD_SUCCESS_BLOCKER,
        BROKER,
    ),
    blocked(
        "runtime_mediator_cases/fork_bypass_reaping.rs::fork_family_bypasses_are_killed_before_success_or_reuse",
        CHILD_LIFECYCLE_BLOCKER,
        BROKER,
    ),
    blocked(
        "runtime_mediator_cases/fork_bypass_reaping.rs::group_leader_joining_existing_pgid_is_reaped_by_direct_pid",
        CHILD_LIFECYCLE_BLOCKER,
        BROKER,
    ),
    blocked(
        "runtime_mediator_cases/fork_bypass_reaping.rs::term_resistant_output_overflow_is_reaped_without_reuse",
        CHILD_LIFECYCLE_BLOCKER,
        BROKER,
    ),
    routed(
        "runtime_mediator_cases/fork_bypass_reaping.rs::mediator_source_keeps_issuance_public_dispatch_and_claims_outside_the_boundary",
        BROKER,
    ),
    routed(
        "runtime_mediator_cases/invocation_fixture.rs::mediator_fixture_catalog_is_exact_and_claimless",
        BINDING,
    ),
    routed(
        "runtime_mediator_cases/invocation_fixture.rs::clean_noop_consumes_no_authority_spawns_nothing_and_writes_nothing",
        NOOP,
    ),
    blocked(
        "runtime_mediator_cases/invocation_fixture.rs::ordered_execution_emits_correlated_results_and_exact_reuse_skips_all_spawns",
        CHILD_SUCCESS_BLOCKER,
        BROKER,
    ),
    routed(
        "runtime_mediator_cases/read_source_refusals.rs::read_source_binding_refuses_symlinks_hardlinks_special_files_and_capture_races",
        READS,
    ),
    routed(
        "runtime_mediator_cases/read_source_refusals.rs::read_source_aba_before_spawn_is_refused_without_effect",
        READS,
    ),
    blocked(
        "runtime_mediator_cases/read_source_refusals.rs::successful_exit_cannot_hide_post_spawn_read_source_mutation",
        CHILD_LIFECYCLE_BLOCKER,
        READS,
    ),
];
