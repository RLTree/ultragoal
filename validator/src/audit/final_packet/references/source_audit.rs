use super::{RefStatusPolicy, load_ref};
use serde_json::{Value, json};
use std::path::Path;

const SELF_REWRITING_SOURCE_AUDIT: &str =
    "validation_artifacts/ultragoal-audit/validator-receipt.json";

pub(super) fn check_ref(root: &Path, receipt: &Value, expected: &str, out: &mut Vec<String>) {
    if let Some(value) = load_ref(
        root,
        receipt,
        "/source_audit",
        "source_audit",
        RefStatusPolicy::MustPass,
        out,
    ) {
        audit_receipt_failures(&value, expected, true, out);
    }
}

pub(super) fn check_guard_ref(root: &Path, receipt: &Value, expected: &str, out: &mut Vec<String>) {
    if let Some(value) = load_guard_ref(root, receipt, expected, out) {
        audit_receipt_guard_failures(&value, expected, out);
    }
}

fn load_guard_ref(
    root: &Path,
    receipt: &Value,
    expected: &str,
    out: &mut Vec<String>,
) -> Option<Value> {
    let Some(item) = receipt.pointer("/source_audit") else {
        out.push("final_packet_proof_ref_missing:source_audit".to_string());
        return None;
    };
    let rel = item.get("path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        out.push(format!(
            "final_packet_proof_ref_path_invalid:source_audit:{rel}"
        ));
        return None;
    }
    let expected_digest = item.get("digest").and_then(Value::as_str).unwrap_or("");
    if self_rewrite_claim_guard_allowed(root, item, rel, expected_digest) {
        return Some(self_rewrite_fail_closed_source_audit(expected));
    }
    let actual = crate::digest::file(&root.join(rel));
    if actual.as_deref() == Ok(expected_digest) {
        return load_ref(
            root,
            receipt,
            "/source_audit",
            "source_audit",
            RefStatusPolicy::PassOrFail,
            out,
        );
    }
    out.push(format!(
        "final_packet_proof_ref_digest_mismatch:source_audit:{rel}"
    ));
    None
}

fn self_rewrite_claim_guard_allowed(
    root: &Path,
    item: &Value,
    rel: &str,
    expected_digest: &str,
) -> bool {
    rel == SELF_REWRITING_SOURCE_AUDIT
        && crate::package::inventory::package_path_error(root, rel).is_none()
        && expected_digest.starts_with("sha256:")
        && expected_digest != crate::digest::ZERO
        && item.get("status").and_then(Value::as_str) == Some("fail")
        && item.get("self_rewriting_authority").and_then(Value::as_str)
            == Some("source_audit_command_writes_validator_receipt")
}

fn self_rewrite_fail_closed_source_audit(expected: &str) -> Value {
    json!({
        "status": "fail",
        "target_revision": {
            "kind": "package_digest",
            "value": expected
        },
        "claim_ceiling": "withheld_or_blocked",
        "supported_claim_classes": [],
        "blocked_claim_classes": [
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "final_packet_correctness",
            "update_goal_eligibility",
            "app_registry_or_reviewer_exposure"
        ]
    })
}

fn audit_receipt_failures(
    value: &Value,
    expected: &str,
    require_pass: bool,
    out: &mut Vec<String>,
) {
    let Some(status) = value.get("status").and_then(Value::as_str) else {
        out.push("final_packet_proof_source_audit_status_missing".to_string());
        return;
    };
    if !matches!(status, "pass" | "fail") {
        out.push(format!(
            "final_packet_proof_source_audit_status_unknown:{status}"
        ));
    }
    if require_pass && status != "pass" {
        out.push("final_packet_proof_source_audit_status_not_pass".to_string());
    }
    if value
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        != Some(expected)
    {
        out.push("final_packet_proof_source_audit_target_digest_mismatch".to_string());
    }
    audit_receipt_pass_claims(value, out);
}

fn audit_receipt_guard_failures(value: &Value, expected: &str, out: &mut Vec<String>) {
    let Some(status) = value.get("status").and_then(Value::as_str) else {
        out.push("final_packet_proof_source_audit_status_missing".to_string());
        return;
    };
    if !matches!(status, "pass" | "fail") {
        out.push(format!(
            "final_packet_proof_source_audit_status_unknown:{status}"
        ));
        return;
    }
    if value
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        != Some(expected)
    {
        out.push("final_packet_proof_source_audit_target_digest_mismatch".to_string());
    }
    if status == "pass" {
        audit_receipt_pass_claims(value, out);
    } else {
        audit_receipt_fail_closed_claims(value, out);
    }
}

fn audit_receipt_pass_claims(value: &Value, out: &mut Vec<String>) {
    if value.get("claim_ceiling").and_then(Value::as_str)
        != Some("source_audit_pass_source_local_only")
    {
        out.push("final_packet_proof_source_audit_claim_ceiling_not_source_local".to_string());
    }
    for supported in ["source_local_audit_checks", "red_fixture_report"] {
        if !array_contains(value, "supported_claim_classes", supported) {
            out.push(format!(
                "final_packet_proof_source_audit_supported_claim_missing:{supported}"
            ));
        }
    }
    audit_receipt_blocked_claims(value, out);
}

fn audit_receipt_fail_closed_claims(value: &Value, out: &mut Vec<String>) {
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("withheld_or_blocked") {
        out.push("final_packet_proof_source_audit_fail_claim_ceiling_not_blocking".to_string());
    }
    if value
        .get("supported_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        out.push("final_packet_proof_source_audit_fail_supported_claims_present".to_string());
    }
    audit_receipt_blocked_claims(value, out);
}

fn audit_receipt_blocked_claims(value: &Value, out: &mut Vec<String>) {
    for blocked in [
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure",
    ] {
        if !array_contains(value, "blocked_claim_classes", blocked) {
            out.push(format!(
                "final_packet_proof_source_audit_missing_blocked_claim:{blocked}"
            ));
        }
    }
}

fn array_contains(value: &Value, key: &str, needle: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(needle)))
}
