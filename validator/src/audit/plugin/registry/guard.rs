use serde_json::Value;
use std::path::Path;

pub(super) fn failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let expected = crate::package::inventory::package_digest(root).unwrap_or_default();
    let actual = receipt
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        .unwrap_or("");
    if actual != expected {
        out.push(format!(
            "plugin_self_law_registry_target_digest_mismatch:{actual}!={expected}"
        ));
    }
    if receipt.get("status").and_then(Value::as_str) != Some("fail") {
        out.push("plugin_self_law_registry_guard_status_not_fail".to_string());
    }
    if receipt.get("claim_ceiling").and_then(Value::as_str) != Some("withheld_or_blocked") {
        out.push("plugin_self_law_registry_guard_claim_ceiling_not_blocking".to_string());
    }
    if string(receipt, "source") != "ultragoal.registry_probe" {
        out.push("plugin_self_law_registry_guard_wrong_source".to_string());
    }
    if string(receipt, "capture_method") != "fail_closed_no_capability" {
        out.push("plugin_self_law_registry_guard_capture_method_not_fail_closed".to_string());
    }
    if receipt.pointer("/issuer/tool").and_then(Value::as_str) != Some("ultragoal") {
        out.push("plugin_self_law_registry_guard_wrong_issuer_tool".to_string());
    }
    if receipt.pointer("/issuer/authority").and_then(Value::as_str) != Some("cli_control_plane") {
        out.push("plugin_self_law_registry_guard_wrong_issuer_authority".to_string());
    }
    if receipt.pointer("/failure/reason").and_then(Value::as_str)
        != Some("live_registry_reviewer_exposure_not_proven")
    {
        out.push("plugin_self_law_registry_guard_failure_reason_missing".to_string());
    }
    for claim in [
        "app_registry_or_reviewer_exposure",
        "review_readiness",
        "release_readiness",
        "completion",
        "update_goal_eligibility",
    ] {
        if !blocked_claims_contain(receipt, claim) {
            out.push(format!(
                "plugin_self_law_registry_guard_missing_blocked_claim:{claim}"
            ));
        }
    }
    out
}

fn blocked_claims_contain(value: &Value, claim: &str) -> bool {
    value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(|claims| claims.iter().any(|item| item.as_str() == Some(claim)))
        || value
            .pointer("/failure/blocked_claim_classes")
            .and_then(Value::as_array)
            .is_some_and(|claims| claims.iter().any(|item| item.as_str() == Some(claim)))
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
