use serde_json::Value;
use std::path::Path;

const REQUIRED_CEILING: &str = "live_registry_reviewer_exposure_proven";

const REQUIRED_REVIEWERS: &[(&str, &str, &str)] = &[
    (
        "harness_contract_claim_falsifier",
        "contract_claim_falsifier",
        "custom-agents/harness-contract-claim-falsifier.toml",
    ),
    (
        "harness_orchestration_recovery_falsifier",
        "orchestration_recovery_falsifier",
        "custom-agents/harness-orchestration-recovery-falsifier.toml",
    ),
    (
        "harness_security_trust_boundary_falsifier",
        "security_trust_boundary_falsifier",
        "custom-agents/harness-security-trust-boundary-falsifier.toml",
    ),
    (
        "harness_product_simplicity_falsifier",
        "product_simplicity_falsifier",
        "custom-agents/harness-product-simplicity-falsifier.toml",
    ),
];

pub(super) fn failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = current_authority_failures(root, receipt);
    out.extend(observation_provenance_failures(root, receipt));
    out.extend(reviewer_failures(receipt));
    out
}

fn current_authority_failures(root: &Path, receipt: &Value) -> Vec<String> {
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
    if receipt.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("plugin_self_law_registry_status_not_pass".to_string());
    }
    if receipt.get("claim_ceiling").and_then(Value::as_str) != Some(REQUIRED_CEILING) {
        out.push("plugin_self_law_registry_claim_ceiling_not_live_surface".to_string());
    }
    if string(receipt, "generated_at") != string(receipt, "captured_at") {
        out.push("plugin_self_law_registry_generated_capture_mismatch".to_string());
    }
    if string(receipt, "source") != "multi_agent_v1.tool_registry" {
        out.push("plugin_self_law_registry_wrong_source".to_string());
    }
    out
}

fn observation_provenance_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.pointer("/issuer/tool").and_then(Value::as_str) != Some("multi_agent_v1") {
        out.push("plugin_self_law_registry_missing_live_tool_issuer".to_string());
    }
    if receipt.pointer("/issuer/authority").and_then(Value::as_str) != Some("tool_registry") {
        out.push("plugin_self_law_registry_missing_live_tool_issuer".to_string());
    }
    if receipt.pointer("/tool_call/name").and_then(Value::as_str)
        != Some("multi_agent_v1.tool_registry")
    {
        out.push("plugin_self_law_registry_missing_tool_call_identity".to_string());
    }
    for ptr in [
        "/tool_call/call_id",
        "/tool_call/arguments_digest",
        "/boundary/account_id",
        "/boundary/workspace_id",
        "/boundary/session_id",
    ] {
        if receipt
            .pointer(ptr)
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            out.push(format!(
                "plugin_self_law_registry_missing_observation_field:{ptr}"
            ));
        }
    }
    if receipt
        .pointer("/boundary/session_id")
        .and_then(Value::as_str)
        != receipt.get("session_id").and_then(Value::as_str)
    {
        out.push("plugin_self_law_registry_boundary_session_mismatch".to_string());
    }
    if string(receipt, "capture_method") != "live_tool_registry_query" {
        out.push("plugin_self_law_registry_capture_method_not_live".to_string());
    }
    out.extend(raw_observation_failures(root, receipt));
    out
}

fn raw_observation_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let Some(raw) = receipt.get("raw_observation") else {
        return vec!["plugin_self_law_registry_raw_observation_missing".to_string()];
    };
    let path = string(raw, "path");
    if !path.starts_with("validation_artifacts/ultragoal-audit/") {
        return vec![format!(
            "plugin_self_law_registry_raw_observation_path_invalid:{path}"
        )];
    }
    if crate::package::inventory::package_path_error(root, path).is_some() {
        return vec![format!(
            "plugin_self_law_registry_raw_observation_path_invalid:{path}"
        )];
    }
    let expected = string(raw, "digest");
    match crate::digest::file(&root.join(path)) {
        Ok(actual) if actual == expected => Vec::new(),
        _ => vec![format!(
            "plugin_self_law_registry_raw_observation_digest_mismatch:{path}"
        )],
    }
}

fn reviewer_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let rows = array(receipt, "agent_types");
    for (agent_type, persona, path) in REQUIRED_REVIEWERS {
        let matches = rows
            .iter()
            .filter(|row| string(row, "agent_type") == *agent_type)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            out.push(format!(
                "plugin_self_law_registry_agent_missing:{agent_type}"
            ));
            continue;
        }
        reviewer_row_failures(matches[0], agent_type, persona, path, &mut out);
    }
    out
}

fn reviewer_row_failures(
    row: &Value,
    agent_type: &str,
    persona: &str,
    path: &str,
    out: &mut Vec<String>,
) {
    if string(row, "persona") != persona || string(row, "custom_agent_path") != path {
        out.push(format!(
            "plugin_self_law_registry_agent_mismatch:{agent_type}"
        ));
    }
    for key in ["disk_cache_synced", "global_toml_present", "exposed"] {
        if row.get(key).and_then(Value::as_bool) != Some(true) {
            out.push(format!(
                "plugin_self_law_registry_agent_not_current:{agent_type}:{key}"
            ));
        }
    }
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
