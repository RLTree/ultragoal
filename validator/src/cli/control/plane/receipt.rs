use super::RECEIPT_SCHEMA;
use serde_json::Value;

pub(crate) fn surface_value_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(RECEIPT_SCHEMA) {
        out.push("cli_control_plane_receipt_wrong_schema".to_string());
    }
    if value.pointer("/issuer/tool").and_then(Value::as_str) != Some("ultragoal") {
        out.push("cli_control_plane_receipt_wrong_issuer".to_string());
    }
    if value.get("operation").and_then(Value::as_str).is_none() {
        out.push("cli_control_plane_receipt_missing_operation".to_string());
    }
    if value
        .get("candidate_digest")
        .and_then(Value::as_str)
        .is_none()
    {
        out.push("cli_control_plane_receipt_missing_candidate_digest".to_string());
    }
    if value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_none()
    {
        out.push("cli_control_plane_receipt_missing_blocked_claims".to_string());
    }
    if value.pointer("/failure/law_id").and_then(Value::as_str) != Some("cli-self-law-compliance")
        && value.get("failure") != Some(&Value::Null)
    {
        out.push("cli_control_plane_receipt_missing_self_law_failure".to_string());
    }
    out
}

pub(crate) fn same_candidate_pass_failures(
    value: &Value,
    expected_candidate: &str,
    expected_operation: &str,
) -> Vec<String> {
    let mut out = surface_value_failures(value);
    let candidate = value
        .get("candidate_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if candidate != expected_candidate {
        out.push(format!(
            "cli_control_plane_receipt_candidate_digest_mismatch:{candidate}!={expected_candidate}"
        ));
    }
    if value.get("operation").and_then(Value::as_str) != Some(expected_operation) {
        out.push(format!(
            "cli_control_plane_receipt_wrong_operation:{expected_operation}"
        ));
    }
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("cli_control_plane_receipt_not_pass".to_string());
    }
    if value
        .pointer("/issuer/self_law_state")
        .and_then(Value::as_str)
        != Some("self_hosted")
    {
        out.push("cli_control_plane_receipt_not_self_hosted".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str)
        != Some("supports_update_goal_eligibility")
    {
        out.push("cli_control_plane_receipt_claim_ceiling_not_update_goal".to_string());
    }
    if !value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        out.push("cli_control_plane_receipt_blocks_claims".to_string());
    }
    if value
        .get("failure")
        .is_some_and(|failure| failure != &Value::Null)
    {
        out.push("cli_control_plane_receipt_pass_has_failure".to_string());
    }
    out
}
