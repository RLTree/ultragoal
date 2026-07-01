use crate::cli::observe;
use serde_json::json;

#[test]
fn exporter_trace_payload_falls_back_to_operation_and_skips_empty_parent() {
    let trace = json!({
        "run_id": "run-edge",
        "correlation_id": "corr-edge",
        "trace_id": "trace-edge",
        "span_id": "span-root",
        "operation": "observe.edge",
        "span_name": "observe.edge",
        "span_kind": "root",
        "status": "pass",
        "law_id": "law-edge",
        "check_id": "check-edge",
        "claim_id": "claim-edge",
        "failure_class": "",
        "why_failed": "",
        "next_repair": "",
        "claim_impact": "test_only",
        "candidate_digest": "sha256:edge",
        "child_spans": [{
            "run_id": "run-edge",
            "correlation_id": "corr-edge",
            "trace_id": "trace-edge",
            "span_id": "span-child",
            "parent_span_id": "",
            "operation": "observe.edge.child",
            "span_kind": "validator_check",
            "status": "pass",
            "law_id": "law-edge",
            "check_id": "check-edge",
            "claim_id": "claim-edge",
            "failure_class": "",
            "why_failed": "",
            "next_repair": "",
            "claim_impact": "test_only",
            "candidate_digest": "sha256:edge"
        }, {
            "run_id": "run-edge",
            "correlation_id": "corr-edge",
            "trace_id": "trace-edge",
            "span_id": "span-child-parented",
            "parent_span_id": "span-root",
            "operation": "observe.edge.parented",
            "span_name": "observe.edge.parented",
            "span_kind": "receipt_deref",
            "status": "pass",
            "law_id": "law-edge",
            "check_id": "check-edge",
            "claim_id": "claim-edge",
            "failure_class": "",
            "why_failed": "",
            "next_repair": "",
            "claim_impact": "test_only",
            "candidate_digest": "sha256:edge"
        }]
    });
    let payload = observe::telemetry::exporter_trace_payload_for_test(&trace);
    let spans = payload
        .pointer("/resourceSpans/0/scopeSpans/0/spans")
        .and_then(|spans| spans.as_array())
        .expect("exported spans");
    let child = spans
        .iter()
        .find(|span| span["name"] == "observe.edge.child")
        .expect("fallback child span name");
    assert!(child.get("parentSpanId").is_none());
    let parented = spans
        .iter()
        .find(|span| span["name"] == "observe.edge.parented")
        .expect("parented child span");
    assert!(parented.get("parentSpanId").is_some());
}
