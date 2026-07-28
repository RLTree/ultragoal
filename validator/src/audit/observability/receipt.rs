use serde_json::Value;
use std::path::Path;

const PROVE_RECEIPT: &str = "validation_artifacts/observability/observe-prove.json";

pub(super) fn check(root: &Path, out: &mut Vec<String>) {
    let candidate = match crate::package::inventory::package_digest(root) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("observability_candidate_digest_unavailable:{err}"));
            return;
        }
    };
    match crate::json_boundary::read_json(&root.join(PROVE_RECEIPT)) {
        Ok(value) => check_value(&value, &candidate, out),
        Err(_) => out.push(format!(
            "observability_missing_live_stack_proof:{PROVE_RECEIPT}"
        )),
    }
}

fn check_value(value: &Value, candidate: &str, out: &mut Vec<String>) {
    if value.get("schema").and_then(Value::as_str) != Some(super::RECEIPT_SCHEMA) {
        out.push("observability_proof_wrong_schema".to_string());
    }
    if value.get("candidate_digest").and_then(Value::as_str) != Some(candidate) {
        out.push("observability_proof_wrong_candidate_digest".to_string());
    }
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("observability_proof_not_passing".to_string());
    }
    for key in [
        "log_stream_digest",
        "metric_snapshot_digest",
        "trace_bundle_digest",
    ] {
        if value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            out.push(format!("observability_proof_missing:{key}"));
        }
    }
}
