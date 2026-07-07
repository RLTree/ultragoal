#[test]
fn raw_authority_scanner_classifies_live_loop_timing_projections_by_product_fields() {
    let graph_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/live_loop/graph/surface_record.rs",
        "use serde_json::{Value,json};\n\
         pub struct LoopValidationSurface;\n\
         pub struct NodeTiming;\n\
         fn surface_record() -> Value {\n\
             let mut object: serde_json::Map<String, Value> = serde_json::Map::new();\n\
             timing_projection_fields::insert(&mut object);\n\
             claim_evaluation::insert(&mut object);\n\
             json!({\"claim_impact\":\"source-local\",\"speedup_ratio\":\"20x\",\"record\":object})\n\
         }\n",
    );
    assert!(graph_projection.is_empty(), "{graph_projection:?}");

    let timing_field_projection =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/cli/live_loop/graph/timing_projection_fields/result.rs",
            "use serde_json::{Value,json};\n\
             pub struct NodeTiming;\n\
             fn insert(object: &mut serde_json::Map<String, Value>) {\n\
                 object.insert(\"product_latency_ms\".to_string(), json!(1));\n\
                 object.insert(\"reconciled_command_duration_ms\".to_string(), json!(1));\n\
                 object.insert(\"telemetry_reconciliation_duration_ms\".to_string(), json!(1));\n\
                 object.insert(\"verified_local_result_digest\".to_string(), json!(\"sha256:test\"));\n\
                 object.insert(\"verified_local_output_digest\".to_string(), json!(\"sha256:test\"));\n\
                 object.insert(\"telemetry_reconciliation_status\".to_string(), json!(\"pass\"));\n\
                 object.insert(\"equivalence_status\".to_string(), json!(\"executed_current_candidate_not_cache_replay\"));\n\
             }\n",
        );
    assert!(
        timing_field_projection.is_empty(),
        "{timing_field_projection:?}"
    );

    let timing_projection_root =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/cli/live_loop/graph/timing_projection_fields/mod.rs",
            "use serde_json::{Map,Value};\n\
             pub struct NodeTiming;\n\
             pub struct LoopValidationSurface;\n\
             fn insert(object: &mut Map<String, Value>) {\n\
                 work::insert(object);\n\
                 result::insert(object);\n\
                 baseline::insert(object);\n\
             }\n",
        );
    assert!(
        timing_projection_root.is_empty(),
        "{timing_projection_root:?}"
    );

    let timing_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/live_loop/nodes/measurement/timing/record.rs",
        "use serde_json::{Value,json};\n\
         pub struct VerifiedLocalProof;\n\
         const NODE_TIMING_REL: &str = \"validation_artifacts/observability/live-loop-node-timing.json\";\n\
         fn node_timing_row() -> Value {\n\
             json!({\"actual_work_duration_ms\":1,\"verified_local_result_digest\":\"sha256:test\",\"timing_source\":NODE_TIMING_REL})\n\
         }\n",
    );
    assert!(timing_projection.is_empty(), "{timing_projection:?}");

    let backend_readiness = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/live_loop/nodes/measurement/observation/live_backend.rs",
        "use serde_json::{Value,json};\n\
         pub enum LiveQueryRoundtrip { Logs }\n\
         fn unavailable() -> Value {\n\
             json!({\"backend_service\":\"victorialogs\",\"backend_readiness_timeout_ms\":500,\"failure_class\":\"live_loop_observability_backend_unavailable\"})\n\
         }\n",
    );
    assert!(backend_readiness.is_empty(), "{backend_readiness:?}");

    let command_observation =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/cli/live_loop/nodes/measurement/observation/event.rs",
            "use serde_json::Value;\n\
             pub struct FullCommandRun;\n\
             pub struct CommandTelemetry;\n\
             fn project() -> Value {\n\
                 let _ = command_receipt_for_candidate();\n\
                 let _ = \"source_local_live_loop_node_observation_only_not_speed_claim\";\n\
                 Value::Null\n\
             }\n",
        );
    assert!(command_observation.is_empty(), "{command_observation:?}");

    let blocker_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/live_loop/blockers.rs",
        "use serde_json::{Value,json};\n\
         fn first_product_blocker(_: &[Value]) -> Value { json!({\"claim_impact\":\"none\"}) }\n\
         fn first_control_board_blocker(_: &Value) -> Value { json!({\"claim_impact\":\"none\"}) }\n\
         fn first_loop_blocker(_: &Value, _: &Value, _: &Value, _: &Value) -> Value { json!({\"claim_impact\":\"none\"}) }\n",
    );
    assert!(blocker_projection.is_empty(), "{blocker_projection:?}");

    let measurement_projection =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/cli/live_loop/nodes/measurement/mod.rs",
            "use serde_json::{Value,json};\n\
             pub struct ChangedInputs;\n\
             pub struct VerifiedLocalProof;\n\
             fn measure_surface() -> Value { let _ = node_timing_row(); let _ = validation_status(); json!({\"status\":\"pass\"}) }\n",
        );
    assert!(
        measurement_projection.is_empty(),
        "{measurement_projection:?}"
    );

    let execution_field_projection =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/cli/live_loop/nodes/measurement/timing/execution_fields.rs",
            "use serde_json::{Value,json};\n\
             pub struct VerifiedLocalProof;\n\
             fn insert(object: &mut serde_json::Map<String, Value>) {\n\
                 object.insert(\"current_input_digest\".to_string(), json!(\"sha256:test\"));\n\
                 object.insert(\"actual_work_duration_ms\".to_string(), json!(1));\n\
                 object.insert(\"telemetry_reconciliation_status\".to_string(), json!(\"partial\"));\n\
                 object.insert(\"command_argv\".to_string(), json!([\"ultragoal\"]));\n\
             }\n",
        );
    assert!(
        execution_field_projection.is_empty(),
        "{execution_field_projection:?}"
    );

    let speed_node_test_row_materialization =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/cli/performance/proof/speed_nodes/test_rows/cache_hit.rs",
            "use serde_json::{Value,json};\n\
             fn verified_cache_row() -> Value {\n\
                 json!({\"proof_kind\":\"verified_cache_hit\",\"cache_equivalence_status\":\"pass\"})\n\
             }\n",
        );
    assert!(
        speed_node_test_row_materialization.is_empty(),
        "{speed_node_test_row_materialization:?}"
    );
}

#[test]
fn raw_authority_scanner_rejects_generic_live_loop_json_projection_rows() {
    for rel in [
        "validator/src/cli/live_loop/graph/surface_record.rs",
        "validator/src/cli/live_loop/blockers.rs",
        "validator/src/cli/live_loop/nodes/measurement/mod.rs",
        "validator/src/cli/live_loop/nodes/measurement/timing/record.rs",
        "validator/src/cli/live_loop/nodes/measurement/timing/execution_fields.rs",
        "validator/src/cli/live_loop/nodes/measurement/observation/event.rs",
    ] {
        let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            rel,
            "use serde_json::{Value,json};\n\
             fn generic_record() -> Value { json!({\"status\":\"pass\"}) }\n",
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("raw_downstream_authority_unclassified")),
            "{rel}: {failures:?}"
        );
    }
}
