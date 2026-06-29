use crate::cli::observe;
use crate::cli::observe::types::ObserveOperation;
use serde_json::json;
use std::fs;
use std::path::Path;

#[test]
fn observability_package_audit_reports_config_receipt_and_registry_edges() {
    let root = super::minimal_root("observe-package-audit");
    fs::create_dir_all(root.join("dev/observability")).expect("obs dir");
    fs::write(
        root.join("dev/observability/compose.yml"),
        "services:\n  victoriametrics:\n    image: victoriametrics/victoria-metrics:latest\n    ports:\n      - \"0.0.0.0:8428:8428\"\n",
    )
    .expect("compose");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({"commands":["observe prove"],"row_requirements":{"log_instrumentation":true}}),
    )
    .expect("inventory");
    write_law_rows(&root);
    let prove_path = root.join("validation_artifacts/observability/observe-prove.json");
    fs::create_dir_all(prove_path.parent().unwrap()).expect("prove parent");
    crate::json_boundary::write_json(
        &prove_path,
        &json!({"schema":"wrong","status":"fail","candidate_digest":"sha256:bad"}),
    )
    .expect("bad proof");
    let failures = crate::audit::observability::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item == "observability_compose_latest_image_forbidden")
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "observability_compose_public_port_binding")
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "observability_proof_wrong_schema")
    );
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_command_inventory_missing:"))
    );
    fs::remove_dir_all(root).expect("cleanup observe package");
}

#[test]
fn observability_stack_error_and_non_stack_edges_are_typed() {
    let root = super::minimal_root("observe-stack-edges");
    let non_stack = observe::stack::run(
        &root,
        &observe::types::ObserveCommand {
            operation: ObserveOperation::Prove,
            receipt: None,
            query: None,
            run_id: None,
            claim_id: None,
            check_id: None,
            law_id: None,
            row_limit: 100,
            byte_limit: 262_144,
            timeout_ms: 5_000,
        },
    )
    .expect("non stack");
    assert_eq!(non_stack["status"], "fail");
    assert_eq!(non_stack["why_failed"], "not a stack operation");
    assert!(
        observe::stack::shell_error_detail_for_test().contains("synthetic stack launch failure")
    );
    fs::remove_dir_all(root).expect("cleanup stack edges");
}

fn write_law_rows(root: &Path) {
    for rel in [
        "templates/agent-standards/enforcement.json",
        "docs/source-obligation-matrix.json",
        "docs/foundational-law-traceability.json",
    ] {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).expect("parent");
        crate::json_boundary::write_json(
            &path,
            &json!({
                "rows":[{"id":crate::audit::observability::LAW}],
                "obligations":[{"id":crate::audit::observability::LAW}],
                "entries":[{"id":crate::audit::observability::LAW}]
            }),
        )
        .expect("law rows");
    }
}
