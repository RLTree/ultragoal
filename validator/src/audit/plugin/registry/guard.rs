use serde_json::Value;
use std::path::Path;

const ACTIVE_RECEIPT: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";

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
    out.extend(capability_gap_failures(receipt));
    out
}

fn capability_gap_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(gap) = receipt.get("capability_gap") else {
        return vec!["plugin_self_law_registry_guard_capability_gap_missing".to_string()];
    };
    if string(gap, "schema") != "harness-ultragoal.capability-gap.v1" {
        out.push("plugin_self_law_registry_guard_capability_gap_schema".to_string());
    }
    if string(gap, "source_session_id") != string(receipt, "session_id") {
        out.push("plugin_self_law_registry_guard_capability_gap_session".to_string());
    }
    if string(gap, "observed_at") != string(receipt, "captured_at") {
        out.push("plugin_self_law_registry_guard_capability_gap_observed_at".to_string());
    }
    if string(gap, "chosen_promotion_artifact") != ACTIVE_RECEIPT {
        out.push("plugin_self_law_registry_guard_capability_gap_artifact".to_string());
    }
    if string(gap, "current_claim_ceiling") != "withheld_or_blocked" {
        out.push("plugin_self_law_registry_guard_capability_gap_ceiling".to_string());
    }
    if string(gap, "disposition") != "open_claim_blocked" {
        out.push("plugin_self_law_registry_guard_capability_gap_disposition".to_string());
    }
    out.extend(capability_gap_anchor_failures(receipt, gap));
    out.extend(capability_gap_required_items(gap));
    out
}

fn capability_gap_anchor_failures(receipt: &Value, gap: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let raw_path = string(
        receipt.pointer("/raw_observation").unwrap_or(&Value::Null),
        "path",
    );
    let raw_digest = string(
        receipt.pointer("/raw_observation").unwrap_or(&Value::Null),
        "digest",
    );
    let gap_path = string(
        gap.pointer("/source_artifact").unwrap_or(&Value::Null),
        "path",
    );
    let gap_digest = string(
        gap.pointer("/source_artifact").unwrap_or(&Value::Null),
        "digest",
    );
    if gap_path != raw_path || gap_digest != raw_digest {
        out.push("plugin_self_law_registry_guard_capability_gap_source_artifact".to_string());
    }
    out
}

fn capability_gap_required_items(gap: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for law in [
        "capability-gap-extraction-harness-capability-promotion",
        "connector-capability-discovery",
        "distribution-sharing-surface-claim-separation",
    ] {
        if !array_contains(gap, "affected_law_ids", law) {
            out.push(format!(
                "plugin_self_law_registry_guard_capability_gap_missing_law:{law}"
            ));
        }
    }
    for claim in [
        "app_registry_or_reviewer_exposure",
        "update_goal_eligibility",
    ] {
        if !array_contains(gap, "affected_claim_ids", claim) {
            out.push(format!(
                "plugin_self_law_registry_guard_capability_gap_missing_claim:{claim}"
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

fn array_contains(value: &Value, key: &str, expected: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
