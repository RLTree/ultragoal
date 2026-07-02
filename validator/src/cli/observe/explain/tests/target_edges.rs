use crate::cli::observe::types;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

fn prepare_root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit")).expect("audit dir");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({"failures":[]}),
    )
    .expect("audit receipt");
    root
}

fn command(run_id: &str) -> types::ObserveCommand {
    crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        run_id.to_string(),
    ])
    .expect("parse")
    .expect("observe")
}

#[test]
fn explain_identifies_stale_missing_and_passed_target_events() {
    let root = prepare_root("observe-explain-target-edges");
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    write_event(
        &root,
        json!({
            "schema": types::EVENT_SCHEMA,
            "run_id": "run-stale",
            "candidate_digest": "sha256:old",
            "operation": "coverage.prove",
            "status": "fail",
            "failure_class": "coverage_digest_mismatch",
            "why_failed": "coverage receipt target digest is stale",
            "where_failed": "coverage.prove.manifest_digest",
            "next_repair": "rerun exact coverage for current candidate",
            "claim_impact": "coverage_failed_blocks_readiness_release_completion_update_goal",
            "artifact_path": "validation_artifacts/coverage",
            "receipt_path": "validation_artifacts/coverage/coverage-receipt.json",
            "duration_ms": 41,
            "worker_count": 2,
            "task_count": 3,
            "queue_depth": 1
        }),
    );
    let stale = super::run(&root, &command("run-stale")).expect("stale explain");
    assert_eq!(stale["status"], "fail");
    assert!(
        stale["explanation"]["known_current_failure"][0]
            .as_str()
            .unwrap()
            .contains("candidate digest mismatch")
    );
    assert_eq!(stale["observed_duration_ms"], 41);
    assert_path_list_contains(
        &stale["explanation"]["implicated_paths"],
        "validation_artifacts/coverage/coverage-receipt.json",
    );

    write_event(
        &root,
        json!({
            "schema": types::EVENT_SCHEMA,
            "run_id": "run-missing-candidate",
            "operation": "coverage.prove",
            "status": "fail",
            "failure_class": "coverage_digest_mismatch",
            "why_failed": "coverage event omitted candidate digest"
        }),
    );
    let missing =
        super::run(&root, &command("run-missing-candidate")).expect("missing candidate explain");
    assert_eq!(
        missing["explanation"]["known_current_failure"][0],
        "observed telemetry candidate digest missing:run-missing-candidate"
    );

    write_query_receipt(
        &root,
        "logs-pass",
        "logs",
        "run-pass",
        &candidate,
        json!({}),
    );
    write_event(
        &root,
        json!({
            "schema": types::EVENT_SCHEMA,
            "run_id": "run-pass",
            "candidate_digest": candidate,
            "operation": "coverage.prove",
            "status": "pass",
            "failure_class": "none",
            "why_failed": "none",
            "where_failed": "none",
            "next_repair": "none",
            "claim_impact": "observability_evidence_only"
        }),
    );
    let passed = super::run(&root, &command("run-pass")).expect("pass explain");
    assert_eq!(passed["status"], "pass");
    assert_eq!(passed["why_failed"], "none");
    assert_eq!(
        passed["explanation"]["known_current_failure"][0],
        "requested telemetry target passed; no failure"
    );
    assert!(
        passed["next_repair"]
            .as_str()
            .unwrap()
            .contains("No repair")
    );
    assert_eq!(
        passed["explanation"]["query_evidence"]["logs"]["status"],
        "pass"
    );
    fs::remove_dir_all(root).expect("cleanup target edges");
}

#[test]
fn explain_reconciles_query_evidence_by_check_operation_and_failure_class() {
    let root = prepare_root("observe-explain-query-evidence");
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    write_event(
        &root,
        json!({
            "schema": types::EVENT_SCHEMA,
            "run_id": "run-query-evidence",
            "candidate_digest": candidate,
            "operation": "coverage.prove",
            "status": "fail",
            "failure_class": "coverage_prove_failure",
            "why_failed": "coverage percent below 100",
            "where_failed": "coverage.prove.uncovered_lines",
            "next_repair": "add behavior coverage for uncovered branch",
            "claim_impact": "coverage_failed_blocks_readiness_release_completion_update_goal",
            "check_id": "coverage-prove",
            "law_id": "exact-coverage-self-law",
            "claim_id": "coverage-complete"
        }),
    );
    write_query_receipt(
        &root,
        "logs-query",
        "logs",
        "unrelated-run",
        &candidate,
        json!({"query":"check_id:coverage-prove","observed_failure_class":"coverage_prove_failure"}),
    );
    write_query_receipt(
        &root,
        "metrics-query",
        "metrics",
        "unrelated-run",
        &candidate,
        json!({
            "query":"sum by (...) (operation=\"coverage.prove\")",
            "metric_failure_class":"coverage_prove_failure",
            "metric_error_count":2
        }),
    );
    write_query_receipt(
        &root,
        "traces-query",
        "traces",
        "run-query-evidence",
        &candidate,
        json!({"observed_failure_class":"coverage_prove_failure"}),
    );

    let receipt = super::run(&root, &command("run-query-evidence")).expect("query evidence");

    for key in ["logs", "metrics", "traces"] {
        assert_eq!(
            receipt["explanation"]["query_evidence"][key]["status"],
            "pass"
        );
    }
    assert_eq!(
        receipt["explanation"]["query_evidence"]["metrics"]["metric_error_count"],
        2
    );
    assert_path_list_contains(
        &receipt["explanation"]["evidence_sources"],
        "metrics query receipt",
    );
    fs::remove_dir_all(root).expect("cleanup query evidence");
}

fn write_event(root: &Path, event: Value) {
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}

fn write_query_receipt(
    root: &Path,
    name: &str,
    kind: &str,
    run_id: &str,
    candidate: &str,
    extra: Value,
) {
    let mut value = json!({
        "schema": types::QUERY_SCHEMA,
        "query_kind": kind,
        "run_id": run_id,
        "candidate_digest": candidate,
        "status": "pass",
        "row_count": 1,
        "why_failed": "none",
        "where_failed": "none",
        "observed_failure_class": "none",
        "metric_failure_class": "none",
        "metric_error_count": 0,
        "query": format!("run_id:{run_id}")
    });
    for (key, item) in extra.as_object().into_iter().flatten() {
        value[key] = item.clone();
    }
    crate::json_boundary::write_json(
        &root.join(format!("validation_artifacts/observability/{name}.json")),
        &value,
    )
    .expect("query receipt");
}

fn assert_path_list_contains(value: &Value, expected: &str) {
    assert!(
        value
            .as_array()
            .expect("array")
            .iter()
            .any(|item| item.as_str() == Some(expected)),
        "{value}"
    );
}
