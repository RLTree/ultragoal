use crate::cli::observe::command::ObserveOperation;
use serde_json::Value;
use std::path::Path;

pub(super) fn current(root: &Path, candidate: &str) -> bool {
    receipt_current(root, ObserveOperation::StackHealth, candidate)
        && receipt_current(root, ObserveOperation::StackSmoke, candidate)
}

fn receipt_current(root: &Path, operation: ObserveOperation, candidate: &str) -> bool {
    let path = root.join(operation.receipt_rel());
    let Ok(value) = crate::json_boundary::read_json(&path) else {
        return false;
    };
    if value.get("schema").and_then(Value::as_str)
        != Some(crate::cli::observe::command::RECEIPT_SCHEMA)
        || value.get("status").and_then(Value::as_str) != Some("pass")
        || value.get("candidate_digest").and_then(Value::as_str) != Some(candidate)
        || value.get("operation").and_then(Value::as_str) != Some(operation.id())
        || value.get("redaction_proof").and_then(Value::as_str) != Some("pass")
        || value.get("retention_bounds_proof").and_then(Value::as_str) != Some("pass")
        || value.get("bounded_output_proof").and_then(Value::as_str) != Some("pass")
    {
        return false;
    }
    let Some(event) = value.get("event") else {
        return false;
    };
    let Some(metric) = value.get("metric") else {
        return false;
    };
    let Some(trace) = value.get("trace") else {
        return false;
    };
    value.get("log_stream_digest").and_then(Value::as_str)
        == Some(crate::digest::canonical_json(event).as_str())
        && value.get("metric_snapshot_digest").and_then(Value::as_str)
            == Some(crate::digest::canonical_json(metric).as_str())
        && value.get("trace_bundle_digest").and_then(Value::as_str)
            == Some(crate::digest::canonical_json(trace).as_str())
}
