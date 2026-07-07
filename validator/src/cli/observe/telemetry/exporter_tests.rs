use super::*;
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
fn metric_export_accepts_single_sample_and_redacts_unbounded_label_chars() {
    let line = metric_line_for_test(&json!({
        "metric_name": "ultragoal_command_duration_ms",
        "metric_value": 42.0,
        "labels": {
            "operation": "source.audit",
            "run_id": "run/not-a-metric-label"
        }
    }));

    assert!(line.starts_with("ultragoal_command_duration_ms{"));
    assert!(line.contains("operation=\"source.audit\""));
    assert!(line.contains("run_id=\"runnot-a-metric-label\""));
    assert!(line.ends_with(" 42\n"));
}

#[test]
fn trace_payload_names_spans_from_operation_and_skips_empty_parent_span() {
    let payload = trace_payload_for_test(&json!({
        "trace_id": "trace-root",
        "span_id": "span-root",
        "operation": "source.audit",
        "run_id": "run-trace",
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
}
