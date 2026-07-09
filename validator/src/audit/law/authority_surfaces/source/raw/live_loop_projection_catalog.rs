pub(in crate::audit::law::authority_surfaces::source::raw) fn required_projection_markers(
    rel: &str,
) -> Option<&'static [&'static str]> {
    match rel {
        "validator/src/cli/live_loop/context.rs" => {
            Some(&["AuditContext", "changed_files_digest", "input_digest"])
        }
        "validator/src/cli/live_loop/graph/mod.rs" => {
            Some(&["LoopValidationSurface", "input_digest", "claim_impact"])
        }
        "validator/src/cli/live_loop/graph/claim_evaluation.rs" => Some(&[
            "LoopValidationSurface",
            "NodeTiming",
            "product_behavior_observed",
            "independent_reconciliation_surface",
        ]),
        "validator/src/cli/live_loop/graph/surface_record.rs" => Some(&[
            "LoopValidationSurface",
            "NodeTiming",
            "timing_projection_fields::insert",
            "claim_evaluation::insert",
            "speedup_ratio",
        ]),
        "validator/src/cli/live_loop/graph/timing_projection_fields/mod.rs" => Some(&[
            "NodeTiming",
            "LoopValidationSurface",
            "work::insert",
            "result::insert",
            "baseline::insert",
        ]),
        "validator/src/cli/live_loop/graph/timing_projection_fields/work.rs" => Some(&[
            "NodeTiming",
            "product_latency_ms",
            "reconciled_command_duration_ms",
            "telemetry_reconciliation_duration_ms",
            "actual_work_duration_ms",
            "work_unit_count",
        ]),
        "validator/src/cli/live_loop/graph/timing_projection_fields/result.rs" => Some(&[
            "NodeTiming",
            "verified_local_result_digest",
            "verified_local_output_digest",
            "telemetry_reconciliation_status",
            "equivalence_status",
        ]),
        "validator/src/cli/live_loop/graph/timing_projection_fields/baseline.rs" => Some(&[
            "NodeTiming",
            "baseline_proof_kind",
            "baseline_invalidation_proof",
            "affected_set_status",
        ]),
        "validator/src/cli/live_loop/blockers.rs" => Some(&[
            "first_product_blocker",
            "first_control_board_blocker",
            "first_loop_blocker",
            "claim_impact",
        ]),
        "validator/src/cli/live_loop/nodes/measurement/mod.rs" => Some(&[
            "ChangedInputs",
            "VerifiedLocalProof",
            "measure_surface",
            "node_timing_row",
            "validation_status",
        ]),
        "validator/src/cli/live_loop/nodes/measurement/timing/execution_fields.rs" => Some(&[
            "VerifiedLocalProof",
            "current_input_digest",
            "actual_work_duration_ms",
            "telemetry_reconciliation_status",
            "command_argv",
        ]),
        "validator/src/cli/live_loop/nodes/measurement/observation/live_backend.rs" => Some(&[
            "LiveQueryRoundtrip",
            "backend_service",
            "backend_readiness_timeout_ms",
            "live_loop_observability_backend_unavailable",
        ]),
        "validator/src/cli/live_loop/nodes/measurement/observation/reconciliation_report.rs" => {
            Some(&[
                "ReconciliationReport",
                "query_receipt_is_claim_observable",
                "roundtrip_durations_ms",
                "first_failed_roundtrip",
            ])
        }
        "validator/src/cli/live_loop/nodes/timing/node_timing.rs" => Some(&[
            "NodeTiming",
            "telemetry_reconciliation",
            "baseline_invalidation_proof",
            "affected_set_status",
        ]),
        "validator/src/cli/live_loop/nodes/measurement/timing/record.rs" => Some(&[
            "VerifiedLocalProof",
            "node_timing_row",
            "actual_work_duration_ms",
            "verified_local_result_digest",
            "NODE_TIMING_REL",
        ]),
        "validator/src/cli/live_loop/nodes/measurement/observation/event.rs" => Some(&[
            "FullCommandRun",
            "CommandTelemetry",
            "command_receipt_for_candidate",
            "source_local_live_loop_node_observation_only_not_speed_claim",
        ]),
        "validator/src/cli/live_loop/receipt.rs" => Some(&[
            "AuditContext",
            "CommandTelemetry",
            "live_loop_hot_repair_feedback",
        ]),
        "validator/src/cli/live_loop/rust_tests/mod.rs" => Some(&[
            "ImpactedRustTestsCommand",
            "source_local_custom_tooling_prerequisite_only",
            "typed_serial_multi_filter_cargo",
            "fake_parallel_cargo_contention_rejected",
        ]),
        _ => None,
    }
}
