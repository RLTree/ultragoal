use super::*;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;

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
        byte_limit: 1024,
        timeout_ms: 1000,
    }
}

#[test]
fn metrics_query_uses_bounded_historical_window() {
    let mut command = command(ObserveOperation::MetricsQuery);
    command.check_id = Some("coverage-prove-observability-binding".to_string());

    let query = query_text(&command);
    assert_eq!(
        query,
        "sum by (__name__,command,operation,status,law_id,check_id,claim_id,surface,failure_class,exporter,saturation_status) (last_over_time({__name__=~\"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth|ultragoal_command_event_unix_seconds\",check_id=\"coverage-prove-observability-binding\"}[5m]))"
    );
}

#[test]
fn metrics_query_does_not_use_run_id_as_label() {
    let mut command = command(ObserveOperation::MetricsQuery);
    command.run_id = Some("run-abc".to_string());

    let query = query_text(&command);
    assert!(!query.contains("run_id=\"run-abc\""));
    assert!(!query.contains("run_id="));
}

#[test]
fn logs_and_traces_can_select_run_and_correlation_without_metric_labels() {
    let mut logs = command(ObserveOperation::LogsQuery);
    logs.run_id = Some("run-abc".to_string());
    logs.correlation_id = Some("corr-abc".to_string());
    assert_eq!(query_text(&logs), "run_id:run-abc correlation_id:corr-abc");

    let mut traces = command(ObserveOperation::TracesQuery);
    traces.run_id = Some("run-abc".to_string());
    traces.correlation_id = Some("corr-abc".to_string());
    assert_eq!(
        query_text(&traces),
        json!({"correlation_id": "corr-abc", "run_id": "run-abc"}).to_string()
    );

    let mut metrics = command(ObserveOperation::MetricsQuery);
    metrics.run_id = Some("run-abc".to_string());
    metrics.correlation_id = Some("corr-abc".to_string());
    let query = query_text(&metrics);
    assert_eq!(
        query,
        "sum by (__name__,command,operation,status,law_id,check_id,claim_id,surface,failure_class,exporter,saturation_status) (last_over_time({__name__=~\"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth|ultragoal_command_event_unix_seconds\"}[5m]))"
    );
    assert!(!query.contains("run_id=\"run-abc\""));
    assert!(!query.contains("correlation_id=\"corr-abc\""));
}

#[test]
fn custom_metrics_query_is_preserved() {
    let mut command = command(ObserveOperation::MetricsQuery);
    command.query = Some("ultragoal_command_total".to_string());

    assert_eq!(query_text(&command), "ultragoal_command_total");
}
