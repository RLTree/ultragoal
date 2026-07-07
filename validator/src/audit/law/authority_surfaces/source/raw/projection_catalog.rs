pub(super) fn required_projection_markers(rel: &str) -> Option<&'static [&'static str]> {
    match rel {
        "validator/src/cli/control/plane/mod.rs" => Some(&[
            "ControlOperation",
            "receipt_from_control_graph",
            "registry::stdout::print",
        ]),
        "validator/src/cli/control/plane/proof/mod.rs" => Some(&[
            "ControlOperation",
            "diagnostic::failure_value",
            "diagnostic::notes",
        ]),
        "validator/src/cli/control/plane/registry/capability/gap.rs" => Some(&[
            "missing_capability_class",
            "affected_claim_ids",
            "current_claim_ceiling",
        ]),
        "validator/src/audit/receipt/scheduler_execution.rs" => Some(&[
            "crate::scheduler::Metrics",
            "metric.to_value",
            "supports_source_local_scheduler_timing_only_not_readiness",
        ]),
        "validator/src/audit/receipt/speed.rs" => Some(&[
            "target_ms",
            "hard_ceiling_ms",
            "source_local_speed_budget_only_not_readiness",
        ]),
        "validator/src/cli/control/plane/emit.rs" => Some(&[
            "ControlOperation",
            "evidence::same_candidate_pass_failures",
            "required_evidence",
        ]),
        "validator/src/cli/control/plane/proof/diagnostic.rs" => Some(&[
            "claim_ceiling_impact",
            "source_install_cache_impact",
            "rerun_command",
        ]),
        "validator/src/cli/final_packet/proof/spans.rs" => {
            Some(&["span_kind", "receipt_deref", "dereferenced_receipt_digest"])
        }
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
        "validator/src/cli/live_loop/graph/timing_projection_fields.rs" => Some(&[
            "NodeTiming",
            "product_latency_ms",
            "reconciled_command_duration_ms",
            "telemetry_reconciliation_duration_ms",
            "verified_local_result_digest",
            "verified_local_output_digest",
            "telemetry_reconciliation_status",
            "equivalence_status",
        ]),
        "validator/src/cli/live_loop/nodes/measurement/observation/backend_readiness.rs" => {
            Some(&[
                "RoundtripQuery",
                "backend_service",
                "backend_readiness_timeout_ms",
                "live_loop_observability_backend_unavailable",
            ])
        }
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
            "observability_live_loop_source_local_increment",
        ]),
        "validator/src/cli/observe/explain/summary.rs" => {
            Some(&["ExplainContext", "smallest_repair", "query_evidence"])
        }
        "validator/src/cli/observe/explain/mod.rs" => Some(&[
            "telemetry::base_receipt",
            "summary::repair_guidance",
            "explanation_target",
        ]),
        "validator/src/cli/observe/query/mod.rs" => Some(&[
            "telemetry::query_result",
            "query_with_retry",
            "result_from_output_for_candidate",
        ]),
        "validator/src/cli/observe/query/text.rs" => Some(&[
            "trace_tags",
            "trace_tag_pairs",
            "serde_json::Map::new",
            "Value::Object",
        ]),
        "validator/src/cli/observe/stack/mod.rs" => {
            Some(&["telemetry::base_receipt", "smoke_receipt", "stack_command"])
        }
        "validator/src/cli/observe/telemetry/claims.rs" => Some(&[
            "ObserveOperation",
            "observability_product_closure_failed_completion_readiness_release_update_goal_blocked",
            "bounds_status",
        ]),
        "validator/src/cli/observe/telemetry/metric.rs" => Some(&[
            "ultragoal_command_total",
            "ultragoal_command_duration_ms",
            "saturation_status",
        ]),
        "validator/src/cli/observe/telemetry/command.rs" => Some(&[
            "CommandTelemetry",
            "query_examples",
            "metric_snapshot_digest",
        ]),
        "validator/src/cli/observe/telemetry/receipt.rs" => {
            Some(&["ObserveCommand", "query_examples", "trace_bundle_digest"])
        }
        "validator/src/cli/observe/telemetry/trace.rs" => {
            Some(&["child_spans", "parent_span_id", "span_kind"])
        }
        "validator/src/cli/performance/proof/speed_nodes/mod.rs" => Some(&[
            "speed_proof_value",
            "current_nodes",
            "first_blocker",
            "performance_claims_withheld_until_speed_nodes_record_product_work_or_verified_reuse",
        ]),
        "validator/src/cli/performance/proof/speed_nodes/claim_readiness.rs" => Some(&[
            "node_supports_positive_speed_claim",
            "node_has_cache_replay_speed_proof",
            "verified_same_candidate_cache_replay",
            "first_blocker",
        ]),
        "validator/src/cli/performance/proof/speed_nodes/timing_projection.rs" => Some(&[
            "current_nodes",
            "project_node",
            "command_argv",
            "NODE_TIMING_REL",
        ]),
        "validator/src/cli/openai/config.rs" => Some(&[
            "openai_config_redacted_resolution",
            "secret_material_serialized",
            "blocked_claims",
        ]),
        "validator/src/audit/law/authority_surfaces/surface_inventory/mod.rs" => Some(&[
            "AuthoritySurfaceInventoryRow",
            "harness-ultragoal.foundational-law-surface-inventory.v1",
            "surface_state",
        ]),
        "validator/src/audit/observability/registry/control.rs" => {
            Some(&["observability_control_board", "Counts", "first_incomplete"])
        }
        "validator/src/cli/observe/command_roundtrip/checks.rs" => Some(&[
            "CommandObservabilitySpec",
            "roundtrip_path",
            "is_command_observable",
        ]),
        "validator/src/cli/observe/command_roundtrip/receipt.rs" => Some(&[
            "CommandRoundtripRecord",
            "CommandObservabilitySpec",
            "spec_driven_observability_command_roundtrip_increment",
        ]),
        "validator/src/cli/product/cohesion.rs" => Some(&[
            "product-cohesion",
            "source_local_product_cohesion_only",
            "target_repo::product::cohesion::check",
        ]),
        "validator/src/cli/review/round.rs" => Some(&[
            "review-round.verify",
            "CommandTelemetry",
            "review_round_source_local_observability",
        ]),
        "validator/src/target_repo/baseline.rs" => {
            Some(&["baseline_checks", "checks: &mut serde_json::Map", "row("])
        }
        "validator/src/target_repo/baseline_mode.rs" => {
            Some(&["mode_checks", "checks: &mut serde_json::Map", "row("])
        }
        "validator/src/target_repo/mod.rs" => {
            Some(&["audit_target_repo", "target_receipt", "display_command"])
        }
        "validator/src/target_repo/receipt.rs" => Some(&[
            "TargetReceiptInput",
            "canonical_fingerprint",
            "repo_fingerprint",
        ]),
        _ => None,
    }
}
