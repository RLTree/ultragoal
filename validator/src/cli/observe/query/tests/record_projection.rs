use crate::cli::observe::query::QueryKind;
use crate::cli::observe::types::ObserveOperation;
use serde_json::json;
use std::path::Path;

#[test]
fn query_receipt_projects_pass_target_identity_from_structured_rows() {
    let root = super::prepare_root("observe-record-projection-pass-target");
    let event = target_event(&root, "pass", "none");
    let body = format!("{}\n{}\n", observe_query_event(&root), event);

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &super::command(),
        QueryKind::Logs,
        "run_id:run-query-bound".to_string(),
        Ok(body),
    )
    .expect("projected query receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["observed_operation"], "source.audit");
    assert_eq!(receipt["observed_status"], "pass");
    assert_eq!(receipt["observed_run_id"], "run-query-bound");
    assert_eq!(receipt["observed_record"]["operation"], "source.audit");
    assert_eq!(receipt["observed_candidate_digest"], current_digest(&root));
    std::fs::remove_dir_all(root).expect("cleanup pass target projection");
}

#[test]
fn query_receipt_projects_failure_target_identity_and_repair_fields() {
    let root = super::prepare_root("observe-record-projection-fail-target");
    let event = target_event(&root, "fail", "source_audit_check_failure");

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &super::command(),
        QueryKind::Logs,
        "run_id:run-query-bound".to_string(),
        Ok(event.to_string()),
    )
    .expect("failure projection receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["observed_operation"], "source.audit");
    assert_eq!(
        receipt["observed_failure_class"],
        "source_audit_check_failure"
    );
    assert_eq!(
        receipt["observed_next_repair"],
        "repair first source audit blocker"
    );
    std::fs::remove_dir_all(root).expect("cleanup failure target projection");
}

#[test]
fn traces_query_projects_span_identity_with_process_candidate_tags() {
    let root = super::prepare_root("observe-record-projection-trace-target");
    target_event(&root, "pass", "none");
    let mut command = super::command();
    command.operation = ObserveOperation::TracesQuery;

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &command,
        QueryKind::Traces,
        "{\"run_id\":\"run-query-bound\"}".to_string(),
        Ok(trace_body(&root).to_string()),
    )
    .expect("trace projection receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["observed_operation"], "source.audit");
    assert_eq!(receipt["observed_status"], "pass");
    assert_eq!(receipt["observed_candidate_digest"], current_digest(&root));
    std::fs::remove_dir_all(root).expect("cleanup trace target projection");
}

fn target_event(root: &Path, status: &str, failure_class: &str) -> serde_json::Value {
    let candidate = current_digest(root);
    let event = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-query-bound",
        "correlation_id": "corr-query-bound",
        "trace_id": "trace-query-bound",
        "span_id": "span-query-bound",
        "candidate_digest": candidate,
        "operation": "source.audit",
        "status": status,
        "failure_class": failure_class,
        "why_failed": if failure_class == "none" { "none" } else { "source audit failed checks: first_check=agent-standards-enforcement" },
        "where_failed": if failure_class == "none" { "none" } else { "source.audit" },
        "next_repair": if failure_class == "none" { "none" } else { "repair first source audit blocker" },
        "claim_impact": "source_audit_failed_blocks_readiness_release_completion_update_goal"
    });
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool target");
    event
}

fn trace_body(root: &Path) -> serde_json::Value {
    json!({
        "data": [{
            "processes": {
                "p1": {
                    "serviceName": "ultragoal",
                    "tags": [{"key": "candidate_digest", "value": current_digest(root)}]
                }
            },
            "spans": [
                {
                    "operationName": "observe.logs.query",
                    "processID": "p1",
                    "tags": [{"key": "operation", "value": "observe.logs.query"}]
                },
                {
                    "operationName": "source.audit",
                    "processID": "p1",
                    "tags": [
                        {"key": "operation", "value": "source.audit"},
                        {"key": "run_id", "value": "run-query-bound"},
                        {"key": "correlation_id", "value": "corr-query-bound"},
                        {"key": "status", "value": "pass"},
                        {"key": "failure_class", "value": "none"},
                        {"key": "why_failed", "value": "none"}
                    ]
                }
            ]
        }],
        "total": 1
    })
}

fn observe_query_event(root: &Path) -> serde_json::Value {
    json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-query-bound",
        "candidate_digest": current_digest(root),
        "operation": "observe.logs.query",
        "status": "pass",
        "failure_class": "none"
    })
}

fn current_digest(root: &Path) -> String {
    crate::package::inventory::package_digest(root).expect("candidate")
}
