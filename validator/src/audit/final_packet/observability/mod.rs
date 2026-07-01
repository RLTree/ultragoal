use serde_json::Value;
use std::path::Path;

#[cfg(test)]
mod tests;

pub(super) fn failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(obs) = receipt.get("observability") else {
        out.push("final_packet_proof_observability_missing".to_string());
        return out;
    };
    let expected = crate::package::inventory::package_digest(root).unwrap_or_default();
    if obs.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.observability-receipt.v1")
    {
        out.push("final_packet_proof_observability_wrong_schema".to_string());
    }
    if obs.get("candidate_digest").and_then(Value::as_str) != Some(expected.as_str()) {
        out.push("final_packet_proof_observability_candidate_digest_mismatch".to_string());
    }
    let receipt_status = receipt.get("status").and_then(Value::as_str);
    if obs.get("status").and_then(Value::as_str) != receipt_status {
        out.push("final_packet_proof_observability_status_mismatch".to_string());
    }
    for key in [
        "run_id",
        "correlation_id",
        "why_failed",
        "where_failed",
        "next_repair",
    ] {
        if obs
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            out.push(format!("final_packet_proof_observability_missing:{key}"));
        }
    }
    if receipt_status == Some("fail")
        && obs.get("why_failed").and_then(Value::as_str) == Some("none")
    {
        out.push("final_packet_proof_observability_empty_failure".to_string());
    }
    check_digest(obs, "log_stream_digest", "/event", &mut out);
    check_digest(obs, "metric_snapshot_digest", "/metric", &mut out);
    check_digest(obs, "trace_bundle_digest", "/trace", &mut out);
    dereferenced_receipt_span_failures(receipt, obs, &mut out);
    out
}

fn check_digest(obs: &Value, key: &str, pointer: &str, out: &mut Vec<String>) {
    let Some(value) = obs.pointer(pointer) else {
        out.push(format!(
            "final_packet_proof_observability_missing:{pointer}"
        ));
        return;
    };
    let expected = crate::digest::canonical_json(value);
    if obs.get(key).and_then(Value::as_str) != Some(expected.as_str()) {
        out.push(format!(
            "final_packet_proof_observability_digest_mismatch:{key}"
        ));
    }
}

fn dereferenced_receipt_span_failures(receipt: &Value, obs: &Value, out: &mut Vec<String>) {
    let Some(children) = obs.pointer("/trace/child_spans").and_then(Value::as_array) else {
        out.push("final_packet_proof_observability_missing:/trace/child_spans".to_string());
        return;
    };
    let root_span = obs.pointer("/trace/span_id").and_then(Value::as_str);
    let trace_id = obs.pointer("/trace/trace_id").and_then(Value::as_str);
    for (label, path) in dereferenced_receipts(receipt) {
        let found = children.iter().any(|span| {
            span.get("span_kind").and_then(Value::as_str) == Some("receipt_deref")
                && span
                    .get("dereferenced_receipt_label")
                    .and_then(Value::as_str)
                    == Some(label.as_str())
                && span.get("receipt_path").and_then(Value::as_str) == Some(path.as_str())
                && span.get("parent_span_id").and_then(Value::as_str) == root_span
                && span.get("trace_id").and_then(Value::as_str) == trace_id
        });
        if !found {
            out.push(format!(
                "final_packet_proof_observability_receipt_span_missing:{label}:{path}"
            ));
        }
    }
}

fn dereferenced_receipts(receipt: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for key in [
        "cli_performance",
        "registry_exposure",
        "source_audit",
        "coverage",
    ] {
        if let Some(path) = receipt
            .pointer(&format!("/{key}/path"))
            .and_then(Value::as_str)
        {
            out.push((key.to_string(), path.to_string()));
        }
    }
    for (index, row) in receipt
        .get("package_receipts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        if let Some(path) = row.get("path").and_then(Value::as_str) {
            out.push((format!("package_receipt_{index}"), path.to_string()));
        }
    }
    out
}
