use super::retired_behavior_routes::{
    CHILD_LIFECYCLE_BLOCKER, CHILD_SUCCESS_BLOCKER, Mapping, blocked, executed, routed,
};

const BROKER_GATE: &[&str] =
    &["broker_binding_controls::root_broker_gate_refuses_before_spawn_and_writes"];
const ISSUER_VISIBILITY: &[&str] =
    &["issuer_api_visibility::sealed_issuer_and_grant_entrypoints_are_not_externally_callable"];
const CAPACITY: &[&str] = &[
    "routine_work::runtime_adapter::production::ledger::tests::protocol_effect_and_consumed_grant_capacity_refuse_real_next_reservation_transactionally",
];
const CONFLICT: &[&str] = &[
    "routine_work::runtime_adapter::production::ledger::tests::expired_and_concurrently_conflicting_preparations_fail_closed",
];
const RECOVERY: &[&str] = &[
    "routine_work::runtime_adapter::production::ledger::tests::staged_publication_recovers_without_reauthorizing_artifact_bytes",
];
const SUBSTITUTION: &[&str] = &[
    "routine_work::runtime_adapter::production::ledger::tests::candidate_source_command_and_cross_attempt_substitution_never_prepare",
];
const STORE: &[&str] = &[
    "authority_ledger_controls::authority_root_and_state_aliases_or_special_objects_refuse_without_rewrite",
    "authority_ledger_controls::authenticated_state_truncate_unknown_duplicate_and_reorder_mutations_refuse",
];
const NOOP: &[&str] = &[
    "authority_ledger_controls::exact_noop_bypasses_authority_initialization_and_workspace_writes",
];
const REDACTION: &[&str] = &[
    "capture::output::secret_safety_tests::secret_bearing_invocation_withholds_every_output_transformation",
];

pub(crate) const MAP: &[Mapping] = &[
    routed(
        "routine_production_authority_cases/authority_redaction.rs::secrets_paths_and_raw_output_are_absent_from_authority_and_diagnostics",
        REDACTION,
    ),
    executed(
        "routine_production_authority_cases/authority_redaction.rs::production_boundary_has_one_sealed_issuer_and_no_test_grant_entrypoint",
        ISSUER_VISIBILITY[0],
        super::issuer_api_visibility::assert_sealed_issuer_and_grant_entrypoints_are_not_externally_callable,
    ),
    blocked(
        "routine_production_authority_cases/authority_redaction.rs::production_child_race_attempt",
        CHILD_LIFECYCLE_BLOCKER,
        BROKER_GATE,
    ),
    routed(
        "routine_production_authority_cases/authority_redaction.rs::two_processes_racing_the_same_protocol_have_exactly_one_winner",
        CONFLICT,
    ),
    routed(
        "routine_production_authority_cases/authority_scenario.rs::production_authority_fixture_catalog_is_exact_and_claimless",
        NOOP,
    ),
    blocked(
        "routine_production_authority_cases/authority_scenario.rs::production_fresh_execution_replay_refusal_and_exact_reuse_are_durable",
        CHILD_SUCCESS_BLOCKER,
        BROKER_GATE,
    ),
    routed(
        "routine_production_authority_cases/authority_scenario.rs::malformed_reuse_refuses_before_any_authority_transition",
        SUBSTITUTION,
    ),
    routed(
        "routine_production_authority_cases/concurrent_reuse_settlement.rs::forged_and_valid_reuse_concurrency_is_order_independent_and_single_transition",
        CONFLICT,
    ),
    routed(
        "routine_production_authority_cases/concurrent_reuse_settlement.rs::concurrent_reuse_at_consumed_grant_capacity_publishes_one_valid_max_state",
        CAPACITY,
    ),
    routed(
        "routine_production_authority_cases/effect_capacity_bounds.rs::protocol_effect_max_minus_one_max_and_max_plus_one_are_fail_closed",
        CAPACITY,
    ),
    routed(
        "routine_production_authority_cases/effect_capacity_bounds.rs::no_op_bypasses_authority_initialization_and_all_writes",
        NOOP,
    ),
    blocked(
        "routine_production_authority_cases/effect_capacity_bounds.rs::reserved_and_started_crashes_require_exact_bounded_recovery",
        CHILD_SUCCESS_BLOCKER,
        RECOVERY,
    ),
    routed(
        "routine_production_authority_cases/effect_capacity_bounds.rs::expired_recovery_authority_is_rejected_from_authenticated_state",
        CONFLICT,
    ),
    blocked(
        "routine_production_authority_cases/effect_capacity_bounds.rs::preterminal_reconciliation_failure_remains_pending_across_reopen",
        CHILD_SUCCESS_BLOCKER,
        RECOVERY,
    ),
    routed(
        "routine_production_authority_cases/invalid_reuse_recovery.rs::invalid_reuse_never_regresses_complete_or_blocks_later_exact_reuse",
        SUBSTITUTION,
    ),
    routed(
        "routine_production_authority_cases/invalid_reuse_recovery.rs::reuse_preauthorization_generation_race_fails_before_reservation_mutation",
        CONFLICT,
    ),
    routed(
        "routine_production_authority_cases/invalid_reuse_recovery.rs::consumed_grant_max_minus_one_max_and_max_plus_one_are_fail_closed",
        CAPACITY,
    ),
    blocked(
        "routine_production_authority_cases/terminal_failure_settlement.rs::failure_and_cancellation_settle_terminally_without_recovery",
        CHILD_LIFECYCLE_BLOCKER,
        BROKER_GATE,
    ),
    routed(
        "routine_production_authority_cases/terminal_failure_settlement.rs::stale_request_and_self_consistent_substitution_refuse_without_hidden_writes",
        SUBSTITUTION,
    ),
    routed(
        "routine_production_authority_cases/terminal_failure_settlement.rs::owner_only_store_rejects_unknown_hardlink_symlink_special_and_root_replacement",
        STORE,
    ),
    routed(
        "routine_production_authority_cases/terminal_failure_settlement.rs::authenticated_state_rejects_truncate_unknown_duplicate_reorder_rollback_and_mutate_restore",
        STORE,
    ),
];
