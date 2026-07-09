use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn receipt_state(root: &Path, rel: &str, candidate: &str) -> Value {
    let path = root.join(rel);
    let Ok(value) = crate::json_boundary::read_json(&path) else {
        return json!({
            "path": rel,
            "status": "missing",
            "current": false,
            "target_digest": "missing",
            "failure_class": "missing_authority_receipt",
            "authority_claim_binding": "none"
        });
    };
    let observed = digest_field(&value).unwrap_or("missing");
    json!({
        "path": rel,
        "status": value.get("status").and_then(Value::as_str).unwrap_or("unknown"),
        "current": observed == candidate,
        "target_digest": observed,
        "failure_class": failure_class(observed, candidate),
        "authority_claim_binding": authority_claim_binding(observed, candidate, &value),
        "first_detail": value.pointer("/failures/0/detail")
            .or_else(|| value.pointer("/details/0"))
            .and_then(Value::as_str)
            .unwrap_or("none")
    })
}

pub(crate) fn digest_field(value: &Value) -> Option<&str> {
    value
        .pointer("/target_revision/value")
        .or_else(|| value.get("target_digest"))
        .or_else(|| value.get("candidate_digest"))
        .and_then(Value::as_str)
}

fn failure_class(observed: &str, candidate: &str) -> &'static str {
    if observed == candidate {
        "receipt_status_or_claim_blocker"
    } else {
        "stale_or_wrong_digest_evidence"
    }
}

fn authority_claim_binding(observed: &str, candidate: &str, value: &Value) -> &'static str {
    if observed == candidate && value.get("status").and_then(Value::as_str) == Some("pass") {
        "source_local_observation_only"
    } else {
        "none"
    }
}
