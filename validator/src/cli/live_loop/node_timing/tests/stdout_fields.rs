use super::super::{telemetry_json, telemetry_query, telemetry_text};
use serde_json::json;

#[test]
fn telemetry_stdout_fields_fall_back_to_cached_reconciliation() {
    let row = json!({
        "telemetry_reconciliation": {
            "status": "pass",
            "reconciliation_mode": "verified_same_candidate_telemetry_reuse",
            "cached_reconciliation": {
                "run_id": "run-cached",
                "correlation_id": "corr-cached",
                "trace_id": "trace-cached",
                "command_observation_receipt": "validation_artifacts/observability/live-loop/commands/fmt.json",
                "logs_query": {"value": {"query": "logs query"}},
                "metrics_query": {"value": {"query": "metrics query"}},
                "traces_query": {"value": {"query": "traces query"}},
                "first_failed_roundtrip": {"roundtrip": "metrics"}
            }
        }
    });

    assert_eq!(telemetry_text(&row, "run_id"), Some("run-cached"));
    assert_eq!(telemetry_text(&row, "correlation_id"), Some("corr-cached"));
    assert_eq!(telemetry_text(&row, "trace_id"), Some("trace-cached"));
    assert_eq!(telemetry_query(&row, "logs_query"), Some("logs query"));
    assert_eq!(
        telemetry_json(&row, "first_failed_roundtrip"),
        "{\"roundtrip\":\"metrics\"}"
    );
}

#[test]
fn telemetry_stdout_fields_read_direct_query_receipt_shape() {
    let row = json!({
        "telemetry_reconciliation": {
            "status": "pass",
            "run_id": "run-direct",
            "logs_query": {"query": "logs direct query"},
            "metrics_query": {"query": "metrics direct query"},
            "traces_query": {"query": "traces direct query"}
        }
    });

    assert_eq!(telemetry_text(&row, "run_id"), Some("run-direct"));
    assert_eq!(
        telemetry_query(&row, "logs_query"),
        Some("logs direct query")
    );
    assert_eq!(
        telemetry_query(&row, "metrics_query"),
        Some("metrics direct query")
    );
    assert_eq!(
        telemetry_query(&row, "traces_query"),
        Some("traces direct query")
    );
}

#[test]
fn telemetry_stdout_fields_do_not_invent_missing_reconciliation() {
    let no_telemetry = json!({});
    assert_eq!(telemetry_text(&no_telemetry, "run_id"), None);
    assert_eq!(telemetry_query(&no_telemetry, "logs_query"), None);
    assert_eq!(
        telemetry_json(&no_telemetry, "first_failed_roundtrip"),
        "unknown"
    );

    let malformed = json!({
        "telemetry_reconciliation": {
            "logs_query": {"value": {"status": "pass"}},
            "first_failed_roundtrip": {"status": "not-json-string-authority"}
        }
    });
    assert_eq!(telemetry_query(&malformed, "logs_query"), None);
    assert!(
        telemetry_json(&malformed, "first_failed_roundtrip").contains("not-json-string-authority")
    );
}
