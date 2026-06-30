use super::*;
use serde_json::json;
use std::fs;

#[test]
fn source_audit_run_reports_observability_write_failure() {
    let root = crate::self_tests::boundaries::support::temp_root("source-audit-run-observe-error");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    write_minimal_law_surfaces(&root);
    fs::create_dir_all(root.join("validation_artifacts")).expect("artifacts");
    fs::write(root.join("validation_artifacts/observability"), "not a dir")
        .expect("observability blocker");
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let err = run(RunArgs {
        root: root.clone(),
        receipt,
        red_report: None,
        target_repo: None,
        mode: "source".to_string(),
        require_observability: false,
        require_product_cohesion: false,
        jobs: None,
    })
    .expect_err("observability write must fail");
    assert!(err.contains("validation_artifacts/observability"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn audit_observability_reports_empty_failure_and_emit_errors() {
    let root = crate::self_tests::boundaries::support::temp_root("audit-observe-edges");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    crate::json_boundary::write_json(&receipt, &json!({"status":"pass","checks":{}}))
        .expect("receipt");
    observability::write_all(&root, &receipt, None, 1, false, None).expect("observe");
    let value =
        crate::json_boundary::read_json(&root.join(observability::SOURCE_RECEIPT)).expect("source");
    assert_eq!(
        value["why_failed"],
        "source audit failed without check details"
    );

    let bad_root = crate::self_tests::boundaries::support::temp_root("audit-observe-no-root");
    let err = observability::emit_receipt(
        &bad_root,
        observability::ReceiptFields {
            command: "ultragoal source",
            subcommand: "audit",
            operation: "source.audit",
            surface: "source",
            check_id: "source-audit-observability-binding",
            claim_id: "source_audit",
            artifact_path: "validation_artifacts/ultragoal-audit",
            receipt_path: observability::SOURCE_RECEIPT,
            status: "fail",
            failure_class: "source_audit_check_failure",
            why_failed: "missing package manifest",
            where_failed: "source.audit",
            next_repair: "create package manifest",
            claim_impact: "source_audit_failed_blocks_claims",
            supported_claims: Vec::new(),
        },
    )
    .expect_err("missing package manifest blocks telemetry receipt");
    assert!(err.contains("plugin-manifest-draft.json"));
    fs::remove_dir_all(root).expect("cleanup");
}

fn write_minimal_law_surfaces(root: &std::path::Path) {
    fs::create_dir_all(root.join("examples/generated")).expect("generated dir");
    crate::json_boundary::write_json(&root.join("schemas/schema-catalog.json"), &json!([]))
        .expect("schema catalog");
    crate::json_boundary::write_json(
        &root.join("docs/mandatory-law-surfaces.json"),
        &json!({"laws":[]}),
    )
    .expect("mandatory laws");
    crate::json_boundary::write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[]}),
    )
    .expect("obligations");
    crate::json_boundary::write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[]}),
    )
    .expect("standards");
    crate::json_boundary::write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]))
        .expect("red fixtures");
}
