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
fn package_digest_command_emits_observability_receipt_contract() {
    let root = super::minimal_root("package-digest-observability");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::PackageDigest,
    })
    .expect("package digest command");
    assert_eq!(code, 0);
    let receipt_path = root.join("validation_artifacts/observability/package-digest.json");
    let receipt = crate::json_boundary::read_json(&receipt_path).expect("package digest receipt");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    assert_eq!(
        receipt["schema"],
        crate::cli::observe::types::RECEIPT_SCHEMA
    );
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["candidate_digest"], candidate);
    assert_eq!(receipt["law_id"], crate::cli::observe::types::LAW_ID);
    assert_eq!(receipt["check_id"], "package-digest-observability-binding");
    assert_eq!(receipt["claim_id"], "source_package_digest");
    assert_eq!(receipt["event"]["operation"], "package.digest");
    assert!(receipt["event"]["duration_ms"].as_u64().unwrap() > 0);
    assert_eq!(receipt["event"]["worker_count"], 1);
    assert_eq!(receipt["event"]["task_count"], 1);
    assert_eq!(receipt["event"]["queue_depth"], 0);
    assert_eq!(receipt["event"]["cache_mode"], "command_receipt_no_cache");
    assert_eq!(
        receipt["event"]["resource_measurement_status"],
        "wall_time_only_cpu_memory_io_unavailable"
    );
    assert_eq!(
        receipt["metric"]["duration_ms"],
        receipt["event"]["duration_ms"]
    );
    assert_eq!(
        receipt["trace"]["worker_count"],
        receipt["event"]["worker_count"]
    );
    let child_spans = receipt["trace"]["child_spans"]
        .as_array()
        .expect("trace child spans");
    assert!(
        child_spans
            .iter()
            .any(|span| span["span_kind"] == "validator_check")
    );
    assert!(child_spans.iter().all(|span| {
        span["parent_span_id"] == receipt["trace"]["span_id"]
            && span["trace_id"] == receipt["trace"]["trace_id"]
    }));
    assert!(
        receipt["supported_claims"]
            .as_array()
            .expect("supported")
            .iter()
            .any(|item| item.as_str() == Some("source_package_digest"))
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|item| item.as_str() == Some("update_goal_eligibility"))
    );
    fs::remove_dir_all(root).expect("cleanup package digest observability");
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
