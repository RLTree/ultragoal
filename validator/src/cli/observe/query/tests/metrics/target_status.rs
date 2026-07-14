use crate::cli::observe::command::ObserveCommand;
use crate::cli::observe::query::QueryKind;
use serde_json::json;
use std::path::Path;

fn command() -> ObserveCommand {
    let mut command = super::super::metrics_command();
    command.run_id = Some("run-metrics-target-status".to_string());
    command.byte_limit = 4096;
    command.timeout_ms = 100;
    command
}

#[test]
fn target_query_preserves_blocked_target_status() {
    let root = super::super::prepare_root("observe-metrics-target-query-blocked");
    write_target_event(&root);

    let query = super::super::super::metrics::target_query(Path::new(&root), &command())
        .expect("target blocked query");

    assert!(query.contains("operation=\"loop.run\""), "{query}");
    assert!(query.contains("status=\"blocked\""), "{query}");
    assert!(!query.contains("status!=\"pass\""), "{query}");
    assert!(!query.contains("status=\"pass\""), "{query}");
    assert!(!query.contains("run_id="), "{query}");
    assert!(!query.contains("correlation_id="), "{query}");
    std::fs::remove_dir_all(root).expect("cleanup blocked target query");
}

#[test]
fn metric_reconciliation_rejects_explicit_query_status_mismatch() {
    let root = super::super::prepare_root("observe-metrics-target-status-mismatch");
    write_target_event(&root);

    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Metrics,
        "custom explicit query".to_string(),
        Ok(metric_body("fail")),
    )
    .expect("status mismatch receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_status_mismatch:fail!=blocked"
    );
    assert_eq!(receipt["metric_status"], "fail");
    std::fs::remove_dir_all(root).expect("cleanup status mismatch");
}

fn write_target_event(root: &Path) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let event = json!({
        "schema": crate::cli::observe::command::EVENT_SCHEMA,
        "run_id": "run-metrics-target-status",
        "candidate_digest": candidate,
        "operation": "loop.run",
        "status": "blocked",
        "failure_class": "hot_loop_observation_snapshot_pending",
        "duration_ms": 10,
        "task_count": 1,
        "queue_depth": 1
    });
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}

fn metric_body(status: &str) -> String {
    json!({
        "status": "success",
        "data": {"result": [{
            "metric": {
                "__name__": "ultragoal_command_total",
                "operation": "loop.run",
                "status": status,
                "failure_class": "hot_loop_observation_snapshot_pending",
                "saturation_status": "serial_command_typed"
            },
            "value": [1, "1"]
        }]}
    })
    .to_string()
}
