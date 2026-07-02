use serde_json::Value;

pub(super) fn validate(explain_evidence: &Value, failures: &mut Vec<String>) {
    if explain_evidence.get("status").and_then(Value::as_str) != Some("pass") {
        failures.push("same_candidate_explain_failure_not_proven".to_string());
        return;
    }
    if explain_evidence
        .get("fallback_used")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        failures.push("explain_failure_used_fallback".to_string());
    }
    let pass_target = explain_evidence
        .get("target_status")
        .and_then(Value::as_str)
        == Some("pass");
    for field in [
        "root_cause",
        "where_failed",
        "why_failed",
        "smallest_repair",
        "narrow_rerun",
        "broad_rerun",
        "claim_ceiling",
    ] {
        let text = explain_evidence
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or("");
        let none_allowed_for_pass_target =
            pass_target && matches!(field, "where_failed" | "why_failed");
        if text.is_empty() || text == "unknown" || (text == "none" && !none_allowed_for_pass_target)
        {
            failures.push(format!("explain_failure_missing_{field}"));
        }
    }
    if explain_evidence
        .get("implicated_paths")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        failures.push("explain_failure_missing_implicated_paths".to_string());
    }
}
