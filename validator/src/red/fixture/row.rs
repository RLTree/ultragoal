use crate::digest;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn result_row(
    root: &Path,
    packet_rel: &str,
    expected: &Value,
    observed: &str,
    status: &str,
    digest_override: Option<&str>,
    observed_check: Option<&str>,
) -> Value {
    let packet_digest = digest_override
        .map(ToOwned::to_owned)
        .or_else(|| digest::file(&root.join(packet_rel)).ok())
        .unwrap_or_else(|| digest::ZERO.to_string());
    let mut row = json!({
        "packet_path": packet_rel,
        "packet_digest": packet_digest,
        "expected_error": expected_error(expected),
        "expected_failing_check": expected_check(expected),
        "observed_error": observed,
        "validator_exit": if observed == "no_failure" { 0 } else { 1 },
        "status": status
    });
    if let Some(observed_check) = observed_check {
        row["observed_failing_check"] = json!(observed_check);
    }
    row
}

pub(crate) fn invalid_row(row: &Value, observed: &str) -> Value {
    let expected = &row["expected_failure"];
    json!({
        "packet_path": row.get("packet_path").and_then(Value::as_str).unwrap_or("<invalid>"),
        "packet_digest": row.get("packet_digest").and_then(Value::as_str).unwrap_or(digest::ZERO),
        "expected_error": expected_error(expected),
        "expected_failing_check": expected_check(expected),
        "observed_error": observed,
        "validator_exit": 1,
        "status": "fail"
    })
}

pub(crate) fn expected_error(value: &Value) -> String {
    value
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("schema_validation_failed")
        .to_string()
}

pub(crate) fn expected_check(value: &Value) -> String {
    value
        .get("check_id")
        .and_then(Value::as_str)
        .unwrap_or("schema-valid")
        .to_string()
}

pub(crate) fn expected_status(expected: &Value, observed: &str) -> &'static str {
    if expected_error(expected) == observed {
        "pass"
    } else {
        "fail"
    }
}
