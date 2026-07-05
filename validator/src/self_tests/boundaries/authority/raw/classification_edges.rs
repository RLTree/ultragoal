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
}

#[test]
fn raw_authority_scanner_rejects_generic_live_loop_json_projection_rows() {
    for rel in [
        "validator/src/cli/live_loop/graph/node_record.rs",
        "validator/src/cli/live_loop/nodes/measurement/timing/record.rs",
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
fn source_text_scanner_routes_raw_and_output_authority_failures() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-source-scanner");
    std::fs::create_dir_all(root.join("validator/src/domain")).expect("source dir");
    std::fs::write(
        root.join("validator/src/domain/claim_core.rs"),
        "use serde_json::Value;\npub(crate) fn decide(value: Value) -> Value { let receipt = root.join(&command.receipt); crate::json_boundary::write_json(&receipt, &value).unwrap(); value }\n",
    )
    .expect("raw source");
    let failures = crate::audit::law::authority_surfaces::source_text_failures_for_test(&root);
    assert!(
        failures.iter().any(|(check, failure)| {
            check == "typed-records-over-prose"
                && failure.contains("raw_downstream_authority_unclassified")
        }),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(check, failure)| {
            check == "total-authority-types-impossible-state-elimination"
                && failure.contains("claim_artifact_output_without_typed_authority")
        }),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup source scanner");
}

#[test]
fn source_text_scanner_excludes_only_test_gated_modules() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-source-scanner-test-gates",
    );
    std::fs::create_dir_all(root.join("validator/src/cli/observe/command_roundtrip"))
        .expect("observe source dir");
    std::fs::create_dir_all(root.join("validator/src/cli/package/inventory"))
        .expect("package source dir");
    std::fs::create_dir_all(root.join("validator/src/cli/current_state"))
        .expect("current state source dir");
    std::fs::create_dir_all(root.join("validator/src/cli/final_packet/proof"))
        .expect("final packet source dir");

    let raw_output_write = "use serde_json::Value;\n\
fn raw_write(root: &std::path::Path, value: &Value) {\n\
    let receipt_path = std::path::PathBuf::from(\"/tmp/claim-output.json\");\n\
    crate::json_boundary::write_json(&receipt_path, value).unwrap();\n\
}\n";
    std::fs::write(
        root.join("validator/src/cli/observe/command_roundtrip/process.rs"),
        format!("pub(crate) fn run() {{}}\n#[cfg(test)]\nmod tests {{\n{raw_output_write}\n}}\n"),
    )
    .expect("inline test module");
    std::fs::write(
        root.join("validator/src/cli/package/inventory/mod.rs"),
        "#[cfg(test)]\nmod command_paths;\n",
    )
    .expect("test sibling parent");
    std::fs::write(
        root.join("validator/src/cli/package/inventory/command_paths.rs"),
        raw_output_write,
    )
    .expect("test sibling module");
    std::fs::write(
        root.join("validator/src/cli/current_state/mod.rs"),
        "mod command_paths;\n",
    )
    .expect("production sibling parent");
    std::fs::write(
        root.join("validator/src/cli/current_state/command_paths.rs"),
        raw_output_write,
    )
    .expect("production sibling module");
    std::fs::write(
        root.join("validator/src/cli/final_packet/proof/observability.rs"),
        "use serde_json::Value;\n\
#[cfg(test)]\n\
mod tests;\n\
pub(crate) fn failures(value: &Value) -> Vec<String> {\n\
    let mut out = Vec::new();\n\
    if value.get(\"status\").and_then(Value::as_str).is_none() {\n\
        out.push(\"status_missing\".to_string());\n\
    }\n\
    out\n\
}\n",
    )
    .expect("declaration test module with production parser after it");

    let failures = crate::audit::law::authority_surfaces::source_text_failures_for_test(&root);
    assert!(
        failures.iter().all(|(_, failure)| !failure
            .contains("validator/src/cli/observe/command_roundtrip/process.rs")),
        "inline cfg(test) modules must not be production authority failures: {failures:?}"
    );
    assert!(
        failures.iter().all(|(_, failure)| !failure
            .contains("validator/src/cli/package/inventory/command_paths.rs")),
        "cfg(test) sibling modules must not be production authority failures: {failures:?}"
    );
    assert!(
        failures.iter().any(
            |(_, failure)| failure.contains("validator/src/cli/current_state/command_paths.rs")
        ),
        "non-test-gated sibling modules must remain production authority surfaces: {failures:?}"
    );
    assert!(
        failures.iter().all(|(_, failure)| !failure
            .contains("validator/src/cli/final_packet/proof/observability.rs")),
        "production parser code after cfg(test) module declarations must remain scanned and classified: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup source scanner test gates");
}
