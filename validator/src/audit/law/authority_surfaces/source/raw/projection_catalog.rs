pub(super) fn required_projection_markers(rel: &str) -> Option<&'static [&'static str]> {
    if let Some(markers) = super::live_loop_projection_catalog::required_projection_markers(rel) {
        return Some(markers);
    }
    match rel {
        "validator/src/cli/final_packet/proof/spans.rs" => {
            Some(&["span_kind", "receipt_deref", "dereferenced_receipt_digest"])
        }
        "validator/src/cli/observe/explain/summary.rs" => {
            Some(&["ExplainContext", "smallest_repair", "query_evidence"])
        }
        "validator/src/cli/observe/explain/mod.rs" => Some(&[
            "telemetry::base_receipt",
            "summary::repair_guidance",
            "explanation_target",
        ]),
        "validator/src/cli/observe/explain/receipt/event_target.rs" => Some(&[
            "fallback_from_receipt",
            "receipt_without_observability_event",
            "fallback_only",
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
        "validator/src/cli/observe/telemetry/exporter.rs" => Some(&[
            "metric::export::lines",
            "trace::export::payload",
            "post_json",
            "post_text",
        ]),
        "validator/src/cli/observe/telemetry/metric/mod.rs" => Some(&[
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
        "validator/src/cli/observe/telemetry/mod.rs" => Some(&[
            "base_receipt_for_candidate",
            "query_result_for_candidate",
            "command_receipt_for_candidate",
        ]),
        "validator/src/cli/observe/telemetry/trace/mod.rs" => {
            Some(&["child_spans", "parent_span_id", "span_kind"])
        }
        "validator/src/cli/observe/telemetry/trace/export.rs" => Some(&[
            "resourceSpans",
            "parentSpanId",
            "child_spans",
            "candidate_digest",
        ]),
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
        "validator/src/audit/law/authority_surfaces/surface_inventory/mod.rs" => {
            Some(&["AuthoritySurfaceInventoryRow"])
        }
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
        "validator/src/cli/observe/command_roundtrip/row.rs" => Some(&[
            "CommandObservabilitySpec",
            "ReconciliationReport",
            "source-local command telemetry roundtrip claim",
            "rejects_generated_rows_only",
        ]),
        "validator/src/cli/product/cohesion.rs" => {
            Some(&["product-cohesion", "source_local_product_cohesion_only"])
        }
        "validator/src/cli/review/round.rs" => Some(&[
            "review-round.verify",
            "CommandTelemetry",
            "review_round_source_local_observability",
        ]),
        _ => None,
    }
}
