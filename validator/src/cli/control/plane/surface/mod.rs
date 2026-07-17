use crate::cli::control::plane::operation::ControlOperation;
use serde_json::Value;

pub(crate) mod target;

pub(crate) const SCHEMA: &str = "harness-ultragoal.package-surface-audit-receipt.v1";
pub(crate) const SCHEMA_FILE: &str = "package-surface-audit-receipt.schema.json";

pub(crate) fn supports(operation: ControlOperation) -> bool {
    matches!(
        operation,
        ControlOperation::InstallAudit | ControlOperation::CacheAudit
    )
}

pub(crate) fn same_candidate_pass_failures(
    value: &Value,
    expected_candidate: &str,
    operation: ControlOperation,
) -> Vec<String> {
    let mut out = surface_failures(value, operation, expected_candidate);
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("package_surface_audit_status_not_pass".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("surface_package_digest_aligned")
    {
        out.push("package_surface_audit_claim_ceiling_not_surface_aligned".to_string());
    }
    if value.get("same_candidate").and_then(Value::as_bool) != Some(true) {
        out.push("package_surface_audit_not_same_candidate".to_string());
    }
    out
}

pub(crate) fn same_candidate_pass_or_fail_closed_failures(
    value: &Value,
    expected_candidate: &str,
    operation: ControlOperation,
) -> Vec<String> {
    if value.get("status").and_then(Value::as_str) == Some("pass") {
        return same_candidate_pass_failures(value, expected_candidate, operation);
    }
    let mut out = base_failures(value, operation, expected_candidate);
    if value.get("status").and_then(Value::as_str) != Some("fail") {
        out.push("package_surface_audit_status_not_pass_or_fail".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("withheld_or_blocked") {
        out.push("package_surface_audit_fail_closed_claim_ceiling_not_blocking".to_string());
    }
    if value.get("same_candidate").and_then(Value::as_bool) != Some(false) {
        out.push("package_surface_audit_fail_closed_same_candidate_not_false".to_string());
    }
    if value
        .get("failures")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push("package_surface_audit_fail_closed_missing_failures".to_string());
    }
    for claim in [
        "install_cache_parity",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "completion",
        "update_goal_eligibility",
    ] {
        if !claim_array_contains(value, "blocked_claim_classes", claim) {
            out.push(format!(
                "package_surface_audit_fail_closed_missing_blocked_claim:{claim}"
            ));
        }
    }
    out
}

pub(crate) fn surface_failures(
    value: &Value,
    operation: ControlOperation,
    expected_candidate: &str,
) -> Vec<String> {
    let mut out = base_failures(value, operation, expected_candidate);
    if value
        .pointer("/target/package_digest")
        .and_then(Value::as_str)
        != Some(expected_candidate)
    {
        out.push("package_surface_audit_target_digest_mismatch".to_string());
    }
    out
}

fn base_failures(
    value: &Value,
    operation: ControlOperation,
    expected_candidate: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push("package_surface_audit_wrong_schema".to_string());
    }
    if value.get("operation").and_then(Value::as_str) != Some(operation.id()) {
        out.push("package_surface_audit_wrong_operation".to_string());
    }
    if value.get("candidate_digest").and_then(Value::as_str) != Some(expected_candidate) {
        out.push("package_surface_audit_candidate_digest_mismatch".to_string());
    }
    if value
        .pointer("/source/package_digest")
        .and_then(Value::as_str)
        != Some(expected_candidate)
    {
        out.push("package_surface_audit_source_digest_mismatch".to_string());
    }
    if value
        .pointer("/target/expected_package_digest")
        .and_then(Value::as_str)
        != Some(expected_candidate)
    {
        out.push("package_surface_audit_target_expected_digest_mismatch".to_string());
    }
    if value.pointer("/target/surface").and_then(Value::as_str)
        != Some(target::surface_id(operation))
    {
        out.push("package_surface_audit_target_surface_mismatch".to_string());
    }
    if value.pointer("/target/local_path").is_some() {
        out.push("package_surface_audit_private_local_path_present".to_string());
    }
    out
}

fn claim_array_contains(value: &Value, key: &str, expected: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

pub(crate) fn value_failures_for_target(
    operation: ControlOperation,
    expected: &str,
    target: &Value,
) -> Vec<String> {
    let mut out = Vec::new();
    if target.get("exists").and_then(Value::as_bool) != Some(true) {
        out.push("package_surface_target_missing".to_string());
    }
    if target.get("surface").and_then(Value::as_str) != Some(target::surface_id(operation)) {
        out.push("package_surface_wrong_surface".to_string());
    }
    if target.get("package_digest").and_then(Value::as_str) != Some(expected) {
        out.push("package_surface_digest_mismatch".to_string());
    }
    if target.get("plugin_name").and_then(Value::as_str) != Some("harness-ultragoal") {
        out.push("package_surface_plugin_name_mismatch".to_string());
    }
    if target.get("plugin_version").and_then(Value::as_str)
        != target.get("source_plugin_version").and_then(Value::as_str)
    {
        out.push("package_surface_plugin_version_mismatch".to_string());
    }
    if target
        .get("package_error")
        .and_then(Value::as_str)
        .is_some()
    {
        out.push("package_surface_digest_unavailable".to_string());
    }
    out
}
