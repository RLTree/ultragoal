use serde_json::{Map, Value, json};

pub(super) fn insert_authority(
    record: &mut Map<String, Value>,
    surface: crate::cli::live_loop::LoopValidationSurface,
    spec: crate::cli::live_loop::SurfaceInputSpec,
    cache_key: &str,
    receipt_path: &str,
) {
    extend(
        record,
        [
            (
                "claim_name",
                json!("source-local live-loop line-cap validation cache"),
            ),
            ("product_behavior_observed", json!(surface.narrow_rerun)),
            (
                "proof_surface",
                json!(
                    "executed strict line-caps command retained for verified-local routine replay"
                ),
            ),
            (
                "independent_reconciliation_surface",
                json!(
                    "line-cap strict stdout, receipt, input digest, cache key, and live-loop replay receipt"
                ),
            ),
            (
                "claim_ceiling",
                json!(
                    "routine line-cap validation reuse only; no readiness release completion final-packet or update_goal claim"
                ),
            ),
            ("affected_set_status", json!("changed_files_digest_bound")),
            ("cache_honesty", json!("pass")),
            (
                "timing_source",
                json!("validation_artifacts/observability/line-cap-check.json"),
            ),
            (
                "claim_impact",
                json!("supports_routine_line_cap_verified_local_reuse_only"),
            ),
            ("validation_status", json!("pass")),
            ("validation_cache_status", json!("reusable")),
            ("observability_status", json!("pass")),
            ("speed_claim_status", json!("withheld")),
            ("observability_failure_class", json!("none")),
            (
                "claim_status",
                json!("withheld_validation_result_available"),
            ),
            ("proof_kind", json!("executed")),
            ("cache_hit", json!(false)),
            ("cache_key", json!(cache_key)),
            ("graph_overhead_ms", json!(1)),
            ("work_unit_count", json!(1)),
            (
                "equivalence_status",
                json!("executed_current_candidate_not_cache_replay"),
            ),
            (
                "invalidation_proof",
                json!(
                    "line_cap_input_digest_command_contract_runtime_model_versions_and_environment_recorded"
                ),
            ),
            ("telemetry_reconciliation_status", json!("pass")),
            (
                "validator_version",
                json!(crate::cli::live_loop::validator_version()),
            ),
            ("law_version", json!(crate::cli::live_loop::law_version())),
            (
                "schema_version",
                json!(crate::cli::live_loop::schema_version()),
            ),
            (
                "fixture_version",
                json!(crate::cli::live_loop::fixture_version()),
            ),
            (
                "runtime_execution_model",
                json!(crate::cli::live_loop::runtime_execution_model()),
            ),
            (
                "surface_input_spec_status",
                json!("surface_input_spec_bound"),
            ),
            ("surface_input_spec_node_id", json!(spec.node_id)),
            (
                "surface_input_spec_cache_boundary",
                json!(spec.cache_boundary_name()),
            ),
            ("validator_authority", json!(spec.validator_authority)),
            ("environment_class", json!(spec.environment_class)),
            ("cache_class", json!(spec.cache_class)),
            ("claim_surface", json!(spec.claim_surface)),
            (
                "output_digest_expectation",
                json!(spec.output_digest_expectation),
            ),
            ("verified_local_command", json!(surface.narrow_rerun)),
            (
                "command_argv",
                json!([
                    "ultragoal",
                    "--root",
                    ".",
                    "line-caps",
                    "check",
                    "--strict",
                    "--jobs",
                    "8"
                ]),
            ),
            ("receipt_paths", json!([receipt_path])),
            ("artifact_paths", json!([receipt_path])),
            ("queue_depth", json!(1)),
            ("worker_count", json!(1)),
            ("task_count", json!(1)),
            ("execution_class", json!("pure_read_parallel")),
            ("exit_status", json!(0)),
            ("verified_local_launch_error", json!(false)),
        ],
    );
}

fn extend<const N: usize>(record: &mut Map<String, Value>, entries: [(&str, Value); N]) {
    for (key, value) in entries {
        record.insert(key.to_string(), value);
    }
}
