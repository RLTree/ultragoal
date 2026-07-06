#[test]
fn raw_authority_scanner_allows_files_without_raw_authority_markers() {
    let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/domain/claim_core.rs",
        "pub(crate) fn typed_claim_status() -> bool { true }\n",
    );
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn raw_authority_scanner_rejects_raw_observation_without_classification() {
    let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/domain/claim_core.rs",
        "pub(crate) fn decide(raw_observation: &str) -> &str { raw_observation }\n",
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_authority=raw_observation")),
        "{failures:?}"
    );
}

#[test]
fn raw_authority_scanner_allows_fixture_and_catalog_materialization_paths() {
    for rel in [
        "fixtures/red/authority-source-binding-red.json",
        "validator/src/self_tests/fixtures/authority.rs",
        "validator/src/audit/fixture/materialization.rs",
        "validator/src/schema_catalog/generated.rs",
        "validator/src/package/schema/catalog.rs",
        "validator/src/cli/schema_catalog.rs",
        "validator/src/audit/namespace/source/rejection_ownership.rs",
        "validator/src/audit/law/authority_surfaces/surface_inventory/discovered/artifact_authority.rs",
        "validator/src/audit/law/authority_surfaces/surface_inventory/discovered/source_authority.rs",
    ] {
        let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            rel,
            "use serde_json::Value;\npub(crate) fn materialize(value: Value) -> Value { value }\n",
        );
        assert!(failures.is_empty(), "{rel}: {failures:?}");
    }
}

#[test]
fn raw_authority_scanner_rejects_directory_only_projection_classification() {
    for rel in [
        "validator/src/cli/observe/receipt/random_projection.rs",
        "validator/src/cli/observe/telemetry/random_projection.rs",
        "validator/src/cli/observe/query/random_projection.rs",
        "validator/src/cli/current_state/random_projection.rs",
    ] {
        let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            rel,
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { json!({\"value\":value}) }\n",
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("raw_downstream_authority_unclassified")),
            "{rel}: {failures:?}"
        );
    }
}

#[test]
fn raw_authority_scanner_allows_typed_map_projection_and_field_parsers() {
    let typed_map = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/domain/claim_projection.rs",
        "use serde_json::Value;\nuse std::collections::BTreeMap;\npub(crate) fn project(value: &Value) -> BTreeMap<&'static str, String> { let mut out = BTreeMap::new(); out.insert(\"status\", value.to_string()); out }\n",
    );
    assert!(typed_map.is_empty(), "{typed_map:?}");

    for parser_text in [
        "use serde_json::Value;\npub(crate) fn failures(value: &Value, out: &mut Vec<String>) { if value.as_object().is_none() { out.push(format!(\"missing object\")); } }\n",
        "use serde_json::Value;\npub(crate) fn failures(value: &Value, out: &mut Vec<String>) { if value.as_array().is_none() { out.push(format!(\"missing array\")); } }\n",
        "use serde_json::Value;\npub(crate) fn failures(value: &Value, out: &mut Vec<String>) { if value.get(\"status\").is_none() { out.push(format!(\"missing status\")); } }\n",
        "use serde_json::Value;\npub(crate) fn failures(value: &Value, out: &mut Vec<String>) { if value.pointer(\"/status\").is_none() { out.push(format!(\"missing status\")); } }\n",
        "use serde_json::Value;\npub(crate) fn failures(value: &Value, out: &mut Vec<String>) { if Value::as_str(value).is_none() { out.push(format!(\"missing str\")); } }\n",
        "use serde_json::Value;\npub(crate) fn str_field(value: &Value, out: &mut Vec<String>) { if str_field_name(value).is_empty() { out.push(format!(\"missing field\")); } }\npub(crate) fn str_field_name(_: &Value) -> &'static str { \"\" }\n",
    ] {
        let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/audit/domain/claim_parser.rs",
            parser_text,
        );
        assert!(failures.is_empty(), "{failures:?}");
    }
}

#[test]
fn raw_authority_scanner_classifies_live_loop_timing_projections_by_product_fields() {
    let graph_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/live_loop/graph/node_record.rs",
        "use serde_json::{Value,json};\n\
         pub struct LoopValidationSurface;\n\
         pub struct NodeTiming;\n\
         fn surface_record() -> Value {\n\
             let mut object: serde_json::Map<String, Value> = serde_json::Map::new();\n\
             object.insert(\"verified_local_result_digest\".to_string(), json!(\"sha256:test\"));\n\
             object.insert(\"telemetry_reconciliation_status\".to_string(), json!(\"pass\"));\n\
             json!({\"claim_impact\":\"source-local\",\"record\":object})\n\
         }\n",
    );
    assert!(graph_projection.is_empty(), "{graph_projection:?}");

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
}

#[test]
fn raw_authority_scanner_classifies_speed_claim_projection_by_product_fields() {
    let speed_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/performance/proof/speed_nodes/mod.rs",
        "use serde_json::{Value,json};\n\
         fn speed_proof_value() -> Value {\n\
             let _ = current_nodes();\n\
             let _ = first_blocker();\n\
             json!({\"claim_impact\":\"performance_claims_withheld_until_speed_nodes_record_product_work_or_verified_reuse\"})\n\
         }\n\
         fn current_nodes() -> Vec<Value> { Vec::new() }\n\
         fn first_blocker() -> Value { Value::Null }\n",
    );
    assert!(speed_projection.is_empty(), "{speed_projection:?}");

    let claim_readiness = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/performance/proof/speed_nodes/claim_readiness.rs",
        "use serde_json::{Value,json};\n\
         fn node_supports_positive_speed_claim(_: &Value) -> bool { true }\n\
         fn node_has_cache_replay_speed_proof(_: &Value) -> bool { true }\n\
         fn first_blocker() -> Value { json!({\"equivalence_status\":\"verified_same_candidate_cache_replay\"}) }\n",
    );
    assert!(claim_readiness.is_empty(), "{claim_readiness:?}");

    let timing_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/performance/proof/speed_nodes/timing_projection.rs",
        "use serde_json::{Value,json};\n\
         const NODE_TIMING_REL: &str = \"validation_artifacts/observability/live-loop-node-timing.json\";\n\
         fn current_nodes() -> Vec<Value> { Vec::new() }\n\
         fn project_node() -> Value { json!({\"command_argv\":[\"ultragoal\"],\"timing_source\":NODE_TIMING_REL}) }\n",
    );
    assert!(timing_projection.is_empty(), "{timing_projection:?}");
}

#[test]
fn raw_authority_scanner_rejects_generic_speed_claim_projection_rows() {
    for rel in [
        "validator/src/cli/performance/proof/speed_nodes/mod.rs",
        "validator/src/cli/performance/proof/speed_nodes/claim_readiness.rs",
        "validator/src/cli/performance/proof/speed_nodes/timing_projection.rs",
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

#[test]
fn raw_authority_scanner_rejects_generic_live_loop_json_projection_rows() {
    for rel in [
        "validator/src/cli/live_loop/graph/node_record.rs",
        "validator/src/cli/live_loop/nodes/measurement/timing/record.rs",
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
