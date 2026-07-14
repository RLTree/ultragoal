use crate::cli::observe::command::{ObserveCommand, ObserveOperation};
use crate::cli::observe::query::QueryKind;
use serde_json::{Value, json};
use std::path::Path;

fn logs_command() -> ObserveCommand {
    let mut command = super::command();
    command.run_id = Some("run-records-reconcile".to_string());
    command.byte_limit = 4096;
    command.timeout_ms = 100;
    command
}

fn traces_command() -> ObserveCommand {
    let mut command = logs_command();
    command.operation = ObserveOperation::TracesQuery;
    command
}

#[test]
fn logs_query_extracts_failure_from_line_delimited_structured_rows() {
    let root = super::prepare_root("observe-records-logs-ndjson");
    let event = write_target_event(&root, "source_audit_check_failure");
    let body = format!("{}\n{}\n", observe_pass_event(&root), event);

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &logs_command(),
        QueryKind::Logs,
        "run_id:run-records-reconcile".to_string(),
        Ok(body),
    )
    .expect("logs receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(
        receipt["observed_failure_class"],
        "source_audit_check_failure"
    );
    assert_eq!(
        receipt["observed_next_repair"],
        "repair first source audit blocker"
    );
    std::fs::remove_dir_all(root).expect("cleanup logs ndjson");
}

#[test]
fn logs_and_traces_fail_when_target_failure_is_missing_or_mismatched() {
    let root = super::prepare_root("observe-records-logs-missing-failure");
    write_target_event(&root, "source_audit_check_failure");
    let receipt = super::super::result_from_output(
        Path::new(&root),
        &logs_command(),
        QueryKind::Logs,
        "run_id:run-records-reconcile".to_string(),
        Ok(observe_pass_event(&root).to_string()),
    )
    .expect("missing failure receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_logs_target_record_missing:run_id=run-records-reconcile"
    );

    let trace_root = super::prepare_root("observe-records-traces-mismatch");
    write_target_event(&trace_root, "source_audit_check_failure");
    let mut mismatched = event_value(&trace_root, "coverage_prove_failure");
    mismatched["operation"] = json!("coverage.prove");
    let receipt = super::super::result_from_output(
        Path::new(&trace_root),
        &traces_command(),
        QueryKind::Traces,
        "{\"run_id\":\"run-records-reconcile\"}".to_string(),
        Ok(mismatched.to_string()),
    )
    .expect("mismatched traces receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_traces_target_record_mismatch:operation:coverage.prove!=source.audit"
    );
    std::fs::remove_dir_all(root).expect("cleanup logs missing");
    std::fs::remove_dir_all(trace_root).expect("cleanup traces mismatch");
}

#[test]
fn logs_query_fails_when_target_event_is_missing_for_requested_run() {
    let root = super::prepare_root("observe-records-target-unavailable");

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &logs_command(),
        QueryKind::Logs,
        "run_id:run-records-reconcile".to_string(),
        Ok(observe_pass_event(&root).to_string()),
    )
    .expect("target unavailable receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_logs_target_unavailable:run_id=run-records-reconcile"
    );
    std::fs::remove_dir_all(root).expect("cleanup records target unavailable");
}

#[test]
fn logs_query_fails_on_target_candidate_mismatch_or_missing_candidate() {
    let root = super::prepare_root("observe-records-candidate-mismatch");
    let mut event = event_value(&root, "source_audit_check_failure");
    event["candidate_digest"] = json!("sha256:old");
    crate::cli::observe::telemetry::spool_write_for_test(&root, &event)
        .expect("spool stale candidate");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &logs_command(),
        QueryKind::Logs,
        "run_id:run-records-reconcile".to_string(),
        Ok(event_value(&root, "source_audit_check_failure").to_string()),
    )
    .expect("candidate mismatch receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        format!("observability_logs_candidate_mismatch:sha256:old!={candidate}")
    );
    std::fs::remove_dir_all(root).expect("cleanup records candidate mismatch");

    let root = super::prepare_root("observe-records-candidate-missing");
    let mut event = event_value(&root, "source_audit_check_failure");
    event.as_object_mut().unwrap().remove("candidate_digest");
    crate::cli::observe::telemetry::spool_write_for_test(&root, &event)
        .expect("spool missing candidate");

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &logs_command(),
        QueryKind::Logs,
        "run_id:run-records-reconcile".to_string(),
        Ok(event_value(&root, "source_audit_check_failure").to_string()),
    )
    .expect("candidate missing receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_logs_candidate_missing"
    );
    std::fs::remove_dir_all(root).expect("cleanup records candidate missing");
}

#[test]
fn logs_query_fails_when_observed_failure_has_no_reason() {
    let root = super::prepare_root("observe-records-why-missing");
    write_target_event(&root, "source_audit_check_failure");
    let mut observed = event_value(&root, "source_audit_check_failure");
    observed["why_failed"] = json!("none");

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &logs_command(),
        QueryKind::Logs,
        "run_id:run-records-reconcile".to_string(),
        Ok(observed.to_string()),
    )
    .expect("missing why receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_logs_why_failed_missing:source_audit_check_failure"
    );
    std::fs::remove_dir_all(root).expect("cleanup records why missing");
}

#[test]
fn logs_query_fails_when_pass_target_rows_contain_failure_signal() {
    let root = super::prepare_root("observe-records-pass-target-failure");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-records-reconcile",
            "candidate_digest": candidate,
            "operation": "source.audit",
            "status": "pass",
            "failure_class": "none",
            "why_failed": "none"
        }),
    )
    .expect("pass target event");

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &logs_command(),
        QueryKind::Logs,
        "run_id:run-records-reconcile".to_string(),
        Ok(event_value(&root, "source_audit_check_failure").to_string()),
    )
    .expect("pass target failure signal receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_logs_pass_target_has_failure_signal:source_audit_check_failure"
    );
    std::fs::remove_dir_all(root).expect("cleanup records pass target failure");
}

fn write_target_event(root: &Path, failure_class: &str) -> Value {
    let event = event_value(root, failure_class);
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
    event
}

fn event_value(root: &Path, failure_class: &str) -> Value {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    json!({
        "schema": crate::cli::observe::command::EVENT_SCHEMA,
        "run_id": "run-records-reconcile",
        "correlation_id": "corr-records-reconcile",
        "candidate_digest": candidate,
        "operation": "source.audit",
        "status": "fail",
        "failure_class": failure_class,
        "why_failed": "source audit failed checks: first_check=agent-standards-enforcement",
        "where_failed": "source.audit",
        "next_repair": "repair first source audit blocker",
        "claim_impact": "source_audit_failed_blocks_readiness_release_completion_update_goal"
    })
}

fn observe_pass_event(root: &Path) -> Value {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    json!({
        "schema": crate::cli::observe::command::EVENT_SCHEMA,
        "run_id": "run-records-reconcile",
        "candidate_digest": candidate,
        "operation": "observe.logs.query",
        "status": "pass",
        "failure_class": "none",
        "why_failed": "none",
        "where_failed": "none",
        "next_repair": "none"
    })
}
