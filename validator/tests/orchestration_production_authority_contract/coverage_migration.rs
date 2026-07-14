use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn retired_direct_route_controls_have_named_production_route_replacements() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/orchestration_production_authority_contract");
    let mappings = [
        (
            "every_adapter_read_projection_is_recursive_zero_write",
            "projection_staleness.rs",
            "current_and_interrupted_projections_are_recursive_zero_write",
        ),
        (
            "sealed_current_view_projects_without_writes_then_resumes_exact_root_state",
            "transaction_binding.rs",
            "one_transaction_cannot_commit_a_while_executing_b",
        ),
        (
            "sealed_ambiguity_view_reconciles_one_exact_operation_and_reopens_cleanly",
            "reconciliation.rs",
            "decision_bound_reconciliation_is_issued_consumed_and_fresh_reopen_is_blocked",
        ),
        (
            "sealed_interrupted_preview_recovers_exact_append_and_reopens_authoritative_head",
            "recovery.rs",
            "interrupted_publication_uses_production_issuance_and_blocks_fresh_reopen",
        ),
        (
            "current_source_action_request_and_permit_substitutions_refuse_before_mutation",
            "execution_binding.rs",
            "wrong_root_binding_target_permits_and_action_refuse_before_reservation",
        ),
        (
            "reconciliation_lease_operation_result_and_source_variant_substitutions_refuse",
            "decision_binding.rs",
            "exact_reconcile_permit_rejects_every_resolution_field_substitution",
        ),
        (
            "interrupted_request_event_binding_tick_workers_and_source_substitutions_refuse",
            "recovery_binding.rs",
            "interrupted_event_binding_tick_workers_and_target_substitutions_do_not_reserve",
        ),
        (
            "stale_view_and_candidate_context_refuse_without_additional_writes",
            "projection_staleness.rs",
            "stale_view_refuses_before_reservation_and_leaves_permit_usable",
        ),
        (
            "exact_resolution_permit_rejects_evidence_outcome_and_every_receipt_field_substitution",
            "decision_binding.rs",
            "exact_reconcile_permit_rejects_every_resolution_field_substitution",
        ),
        (
            "v1_missing_or_mismatched_decision_bindings_fail_closed_without_writes",
            "decision_binding.rs",
            "permit_downgrade_cross_binding_forgery_and_debug_leak_fail_closed",
        ),
        (
            "action_and_reconciliation_permits_cannot_cross_interfaces",
            "authority_boundary.rs",
            "cross_authority_attempt_and_replay_cannot_bypass_one_use_ledger",
        ),
        (
            "permit_debug_and_refusal_diagnostics_do_not_echo_authority_material",
            "decision_binding.rs",
            "permit_downgrade_cross_binding_forgery_and_debug_leak_fail_closed",
        ),
        (
            "concurrent_recovery_has_one_authoritative_winner_and_replay_refuses",
            "recovery_binding.rs",
            "concurrent_recovery_has_one_production_authority_winner",
        ),
        (
            "same_view_differently_bound_reconciliations_have_one_current_head_winner",
            "execution_binding.rs",
            "concurrent_clone_substitution_and_wrong_request_never_reserve_origin",
        ),
        (
            "interrupted_root_inspect_authorize_execute_and_reopen_cross_processes",
            "race.rs",
            "separate_processes_cannot_reopen_nonempty_state_without_external_custody",
        ),
        (
            "ambiguity_inspect_authorize_reconcile_and_reopen_cross_processes",
            "causal_reconciliation.rs",
            "nonempty_reopen_requires_unimplemented_external_monotonic_custody",
        ),
        (
            "interrupted_append_inspect_authorize_recover_and_reopen_cross_processes",
            "recovery.rs",
            "interrupted_publication_uses_production_issuance_and_blocks_fresh_reopen",
        ),
    ];
    assert_eq!(
        mappings
            .iter()
            .map(|mapping| mapping.0)
            .collect::<BTreeSet<_>>()
            .len(),
        mappings.len()
    );
    for (retired, file, replacement) in mappings {
        let source = fs::read_to_string(root.join(file)).unwrap();
        assert!(
            source.contains(&format!("fn {replacement}(")),
            "{retired} -> {replacement}"
        );
    }
}
