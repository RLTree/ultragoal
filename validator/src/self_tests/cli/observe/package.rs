use crate::cli::observe;
use crate::cli::observe::types::ObserveOperation;
use serde_json::json;
use std::fs;
use std::path::Path;

#[test]
fn observability_package_audit_deauthorizes_static_inventory_and_preserves_other_failures() {
    let root = super::minimal_root("observe-package-audit");
    fs::create_dir_all(root.join("dev/observability")).expect("obs dir");
    fs::write(
        root.join("dev/observability/compose.yml"),
        "services:\n  victoriametrics:\n    image: victoriametrics/victoria-metrics:latest\n    ports:\n      - \"0.0.0.0:8428:8428\"\n",
    )
    .expect("compose");
    let static_inventory = root.join("docs/generated/observability/command-inventory.json");
    fs::create_dir_all(static_inventory.parent().expect("inventory parent"))
        .expect("inventory parent");
    fs::write(&static_inventory, "SECRET_CANARY").expect("static bait");
    write_law_rows(&root);
    let prove_path = root.join("validation_artifacts/observability/observe-prove.json");
    fs::create_dir_all(prove_path.parent().unwrap()).expect("prove parent");
    crate::json_boundary::write_json(
        &prove_path,
        &json!({"schema":"wrong","status":"fail","candidate_digest":"sha256:bad"}),
    )
    .expect("bad proof");
    let failures = crate::audit::observability::package_failures(&root);
    fs::write(&static_inventory, [0xff, 0xfe]).expect("invalid static bait");
    let mutated_failures = crate::audit::observability::package_failures(&root);
    fs::remove_file(&static_inventory).expect("remove static bait");
    let missing_failures = crate::audit::observability::package_failures(&root);
    assert_eq!(failures, mutated_failures);
    assert_eq!(mutated_failures, missing_failures);
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
            .any(|item| item == "HCT-OBSERVE successor catalog unavailable/not adopted")
    );
    assert!(!failures.join("\n").contains("SECRET_CANARY"));
    fs::remove_dir_all(root).expect("cleanup observe package");
}

#[test]
fn observability_package_audit_rejects_private_path_spool_leak() {
    let root = super::minimal_root("observe-package-redaction");
    let spool = root.join("validation_artifacts/observability/spool/events.jsonl");
    fs::create_dir_all(spool.parent().unwrap()).expect("spool parent");
    let private_socket = format!(
        "{}{}",
        concat!("unix://", "/", "Users/"),
        "terrynoblin/.docker/run/docker.sock"
    );
    fs::write(
        &spool,
        format!(r#"{{"why_failed":"dial unix {private_socket}","redaction_status":"pass"}}"#),
    )
    .expect("leaky spool");
    let failures = crate::audit::observability::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_private_path_leak:")),
        "{failures:?}"
    );
    fs::write(
        &spool,
        r#"{"why_failed":"dial unix [redacted-home-path]","redaction_status":"pass"}"#,
    )
    .expect("redacted spool");
    let redacted_failures = crate::audit::observability::package_failures(&root);
    assert!(
        !redacted_failures
            .iter()
            .any(|item| item.starts_with("observability_private_path_leak:")),
        "{redacted_failures:?}"
    );
    fs::remove_dir_all(root).expect("cleanup observe redaction audit");
}

#[test]
fn observability_package_audit_rejects_secret_and_unreadable_artifacts() {
    let root = super::minimal_root("observe-package-redaction-secret");
    let artifact_dir = root.join("validation_artifacts/observability/spool");
    fs::create_dir_all(&artifact_dir).expect("spool parent");
    fs::write(
        artifact_dir.join("events.jsonl"),
        r#"{"why_failed":"Authorization: bearer [redacted]","redaction_status":"pass"}"#,
    )
    .expect("secret marker spool");
    fs::write(artifact_dir.join("invalid.jsonl"), [0xff, 0xfe]).expect("invalid utf8 spool");
    let failures = crate::audit::observability::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_secret_marker_leak:")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_artifact_read_error:")),
        "{failures:?}"
    );
    fs::remove_dir_all(root).expect("cleanup observe redaction secret audit");
}

#[test]
fn observability_package_audit_rejects_artifact_root_that_is_not_directory() {
    let root = super::minimal_root("observe-package-redaction-root");
    let artifact_root = root.join("validation_artifacts/observability");
    fs::create_dir_all(artifact_root.parent().unwrap()).expect("artifact parent");
    fs::write(&artifact_root, "not a directory").expect("artifact file");
    let failures = crate::audit::observability::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_artifact_read_error:")),
        "{failures:?}"
    );
    fs::remove_dir_all(root).expect("cleanup observe redaction root audit");
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
            correlation_id: None,
            claim_id: None,
            check_id: None,
            law_id: None,
            target_command: None,
            target_family: None,
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
