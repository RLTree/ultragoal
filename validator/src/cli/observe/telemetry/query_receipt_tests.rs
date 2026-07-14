use super::*;
use crate::cli::observe::command::{ObserveCommand, ObserveOperation};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

fn command(operation: ObserveOperation) -> ObserveCommand {
    ObserveCommand {
        operation,
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
        byte_limit: 4096,
        timeout_ms: 1000,
    }
}

fn temp_root(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = NEXT_TEMP_ROOT.fetch_add(1, Ordering::SeqCst);
    std::env::current_dir()
        .expect("cwd")
        .join("target")
        .join(format!(
            "ultragoal-observe-telemetry-{name}-{}-{stamp}-{sequence}",
            std::process::id()
        ))
}

fn write_minimal_manifest(root: &std::path::Path) {
    std::fs::create_dir_all(root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
}

fn synthetic_private_path() -> String {
    ["", "Users", "tree", "private-token"].join("/")
}

#[test]
fn query_receipts_fail_closed_for_bounds_or_redaction_violation() {
    let root = temp_root("query-proof-shaped-bounds");
    write_minimal_manifest(&root);

    let mut unbounded = command(ObserveOperation::LogsQuery);
    unbounded.timeout_ms = 60_000;
    let bounds_receipt = query_result(
        &root,
        &unbounded,
        "logs",
        "run_id:run-bounds".to_string(),
        vec![json!({"body":"{\"status\":\"pass\"}"})],
        "pass",
        None,
    )
    .expect("bounds receipt");
    assert_eq!(bounds_receipt["status"], "fail");
    assert_eq!(
        bounds_receipt["failure"],
        "observability_query_bounds_failed"
    );
    assert_eq!(bounds_receipt["bounded_output_status"], "fail");
    assert_eq!(
        bounds_receipt["claim_impact"],
        "observability_claims_blocked"
    );

    let redaction_receipt = query_result(
        &root,
        &command(ObserveOperation::LogsQuery),
        "logs",
        "run_id:run-redaction".to_string(),
        vec![json!({"body": synthetic_private_path()})],
        "pass",
        None,
    )
    .expect("redaction receipt");
    assert_eq!(redaction_receipt["status"], "fail");
    assert_eq!(
        redaction_receipt["failure"],
        "observability_query_redaction_failed"
    );
    assert_eq!(redaction_receipt["redaction_status"], "fail");
    assert_eq!(redaction_receipt["supported_claims"], json!([]));
    std::fs::remove_dir_all(root).expect("cleanup query fail closed");
}

#[test]
fn query_receipts_project_specific_failure_class_to_stdout_surface() {
    let root = temp_root("query-specific-failure-class");
    write_minimal_manifest(&root);

    let receipt = query_result(
        &root,
        &command(ObserveOperation::MetricsQuery),
        "metrics",
        "sum by (...)".to_string(),
        vec![],
        "fail",
        Some("observability query returned no matching rows"),
    )
    .expect("query receipt");

    assert_eq!(
        receipt["failure_class"],
        "observability_metric_missing_for_target"
    );
    assert_eq!(
        receipt["failure"],
        "observability query returned no matching rows"
    );
    assert!(
        receipt["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("metric ingestion latency")
    );
    std::fs::remove_dir_all(root).expect("cleanup query failure class");
}

#[test]
fn read_only_query_receipt_ignores_unwritable_spool_authority() {
    let root = temp_root("query-base-receipt-spool-blocked");
    write_minimal_manifest(&root);
    std::fs::create_dir_all(root.join("validation_artifacts/observability"))
        .expect("observability artifact parent");
    std::fs::write(
        root.join("validation_artifacts/observability/spool"),
        b"not a dir",
    )
    .expect("blocked spool path");

    let receipt = query_result_for_candidate(
        &root,
        &command(ObserveOperation::LogsQuery),
        "logs",
        "run_id:run-missing-package".to_string(),
        vec![],
        "fail",
        Some("observability query returned no matching rows"),
        "sha256:test-candidate".to_string(),
    )
    .expect("read-only query result");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        std::fs::read(root.join("validation_artifacts/observability/spool")).expect("blocker"),
        b"not a dir"
    );
    assert!(
        !root
            .join("validation_artifacts/observability/spool/events.jsonl")
            .exists()
    );
    std::fs::remove_dir_all(root).expect("cleanup blocked spool");
}
