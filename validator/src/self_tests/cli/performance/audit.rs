use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn performance_audit_binds_rows_inventory_catalog_and_receipt() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("cli-performance-audit");
    for rel in [
        "validator/src/cli/performance/mod.rs",
        "validator/src/cli/performance/types.rs",
        "schemas/cli-performance-receipt.schema.json",
    ] {
        std::fs::create_dir_all(root.join(rel).parent().expect("parent")).expect("parent dir");
        std::fs::write(root.join(rel), rel).expect("artifact");
    }
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[
            "validator/src/cli/performance/mod.rs",
            "validator/src/cli/performance/types.rs",
            "schemas/cli-performance-receipt.schema.json",
            "validation_artifacts/cli/performance-receipt.json"
        ]}),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[{"path":"schemas/cli-performance-receipt.schema.json"}]}),
    );
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id":"cli-performance-latency-speed-iteration-fitness"}]}),
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{"id":"cli-performance-latency-speed-iteration-fitness"}]}),
    );
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{"obligation_id":"cli-performance-latency-speed-iteration-fitness"}]}),
    );
    write_json(
        &root.join("fixtures/mandatory-law-surfaces/valid/cli-performance-latency-speed-iteration-fitness.json"),
        &json!({"law_id":"cli-performance-latency-speed-iteration-fitness"}),
    );
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));
    write_json(
        &root.join("validation_artifacts/cli/performance-receipt.json"),
        &json!({}),
    );
    let failures = crate::audit::cli::performance::package_failures(&root);
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("missing_standards_row")
                || failure.contains("missing_source_obligation")
                || failure.contains("missing_foundational_trace")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure == "cli_performance_receipt_wrong_schema"),
        "{failures:?}"
    );
    std::fs::remove_file(root.join("validation_artifacts/cli/performance-receipt.json"))
        .expect("remove receipt");
    let missing = crate::audit::cli::performance::package_failures(&root);
    assert!(
        missing
            .iter()
            .any(|failure| { failure.starts_with("cli_performance_missing_fail_closed_receipt:") })
    );
    std::fs::remove_dir_all(root).expect("cleanup cli performance audit");
}
