use super::super::metric_summary;
use serde_json::json;

#[test]
fn metric_summary_rejects_high_cardinality_labels_and_ignores_bad_rows() {
    let rows = vec![
        json!({"body": "not-json"}),
        json!({"no_body": true}),
        json!({"body": json!({
            "data": {"result": [
                {
                    "metric": {
                        "__name__": "ultragoal_command_total",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure",
                        "saturation_status": "queue_pressure",
                        "run_id": "run-high-cardinality"
                    },
                    "value": [10, "2"]
                },
                {
                    "metric": {
                        "__name__": "ultragoal_command_duration_ms",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure"
                    },
                    "value": [10.6, "42.4"]
                },
                {
                    "metric": {
                        "__name__": "ultragoal_command_task_count",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure"
                    },
                    "value": ["11", "3"]
                },
                {
                    "metric": {
                        "__name__": "ultragoal_command_duration_ms",
                        "operation": "coverage.prove",
                        "status": "pass",
                        "failure_class": "none"
                    },
                    "value": [11.5, "12"]
                },
                {
                    "metric": {
                        "__name__": "ultragoal_command_queue_depth",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure",
                        "saturation_status": "queue_pressure_after_join"
                    },
                    "value": [12, "7"]
                },
                {
                    "metric": {
                        "__name__": "ultragoal_command_event_unix_seconds",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure"
                    },
                    "value": [12, "12"]
                }
            ]}
        }).to_string()}),
    ];

    let summary = metric_summary(&rows);

    assert_eq!(summary["operation"], "coverage.prove");
    assert_eq!(summary["status"], "mixed");
    assert_eq!(summary["traffic_count"], 2);
    assert_eq!(summary["error_count"], 2);
    assert_eq!(summary["failure_class"], "coverage_prove_failure");
    assert_eq!(summary["latency_ms"], 42);
    assert_eq!(summary["task_count"], 3);
    assert_eq!(summary["queue_depth"], 7);
    assert_eq!(summary["latest_sample_unix"], 12);
    assert_eq!(summary["event_unix_seconds"], 12);
    assert_eq!(
        summary["saturation_status"],
        "queue_pressure_after_join;queue_depth=7"
    );
    assert_eq!(summary["high_cardinality_labels"], "fail");
}
