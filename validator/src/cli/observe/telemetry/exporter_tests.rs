use super::*;
use serde_json::json;
use std::os::unix::process::ExitStatusExt;

#[test]
fn post_output_result_accepts_successful_export_process() {
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };

    post_output_result(Ok(output)).expect("successful export");
}

#[test]
fn prometheus_sample_keeps_only_bounded_cardinality_labels() {
    let line = metric_line_for_test(&json!({
        "metric_name": "ultragoal_command_duration_ms",
        "metric_value": 42.0,
        "labels": {
            "operation": "source.audit",
            "run_id": "run/not-a-metric-label",
            "candidate_digest": "sha256:not-a-metric-label"
        }
    }));

    assert!(line.starts_with("ultragoal_command_duration_ms{"));
    assert!(line.contains("operation=\"source.audit\""));
    assert!(!line.contains("run_id="));
    assert!(!line.contains("candidate_digest="));
    assert!(line.ends_with(" 42\n"));
}

#[test]
fn prometheus_sample_keeps_event_time_as_value_and_sample_timestamp() {
    let line = metric_line_for_test(&json!({
        "metric_name": "ultragoal_command_event_unix_seconds",
        "metric_value": 1782934959.0,
        "timestamp": "2026-07-01T19:42:39Z",
        "labels": {
            "operation": "source.audit"
        }
    }));

    assert!(line.ends_with(" 1782934959 1782934959000\n"));
}

#[test]
fn trace_payload_names_spans_from_operation_and_skips_empty_parent_span() {
    let payload = trace_payload_for_test(&json!({
        "trace_id": "trace-root",
        "span_id": "span-root",
        "operation": "source.audit",
        "run_id": "run-trace",
        "correlation_id": "corr-trace",
        "candidate_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "child_spans": [{
            "span_id": "child",
            "parent_span_id": "",
            "operation": "source.audit.check",
            "run_id": "run-trace"
        }]
    }));

    let spans = payload["resourceSpans"][0]["scopeSpans"][0]["spans"]
        .as_array()
        .expect("spans");
    assert_eq!(spans[0]["name"], "source.audit");
    assert_eq!(spans[1]["name"], "source.audit.check");
    assert!(spans[1].get("parentSpanId").is_none());
    let resource_attrs = payload["resourceSpans"][0]["resource"]["attributes"]
        .as_array()
        .expect("resource attrs");
    assert!(resource_attrs.iter().any(|attr| {
        attr["key"] == "correlation_id" && attr["value"]["stringValue"] == "corr-trace"
    }));
}
