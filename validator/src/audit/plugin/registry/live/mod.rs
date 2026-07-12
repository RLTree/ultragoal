use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod raw;
mod reviewers;

const REQUIRED_CEILING: &str = "withheld_or_blocked";

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
        out.push("plugin_self_law_registry_target_digest_mismatch".to_string());
    }
    if receipt.get("status").and_then(Value::as_str) != Some("fail") {
        out.push("plugin_self_law_registry_positive_status_forbidden".to_string());
    }
    if receipt.get("claim_ceiling").and_then(Value::as_str) != Some(REQUIRED_CEILING) {
        out.push("plugin_self_law_registry_positive_claim_ceiling_forbidden".to_string());
    }
    if string(receipt, "generated_at") != string(receipt, "captured_at") {
        out.push("plugin_self_law_registry_generated_capture_mismatch".to_string());
    }
    if string(receipt, "source") != "ultragoal.registry_probe" {
        out.push("plugin_self_law_registry_wrong_source".to_string());
    }
    out
}

fn observation_provenance_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.pointer("/issuer/tool").and_then(Value::as_str) != Some("ultragoal") {
        out.push("plugin_self_law_registry_fail_closed_issuer_mismatch".to_string());
    }
    if receipt.pointer("/issuer/authority").and_then(Value::as_str) != Some("cli_control_plane") {
        out.push("plugin_self_law_registry_fail_closed_issuer_mismatch".to_string());
    }
    if receipt.pointer("/tool_call/name").and_then(Value::as_str)
        != Some("ultragoal registry probe")
    {
        out.push("plugin_self_law_registry_fail_closed_tool_call_mismatch".to_string());
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
    if string(receipt, "capture_method") != "fail_closed_no_capability" {
        out.push("plugin_self_law_registry_capture_method_not_fail_closed".to_string());
    }
    out.extend(raw::observation_failures(root, receipt));
    out
}

fn reviewer_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let rows = array(receipt, "agent_types");
    if rows.len() != reviewers::REQUIRED_REVIEWERS.len() {
        out.push("plugin_self_law_registry_agent_count_mismatch".to_string());
    }
    let mut paths = BTreeSet::new();
    for row in &rows {
        reject_legacy_keys(row, &mut out);
        let role = string(row, "role");
        let Some((_, expected_path)) = reviewers::REQUIRED_REVIEWERS
            .iter()
            .find(|(expected_role, _)| *expected_role == role)
        else {
            out.push("plugin_self_law_registry_agent_unexpected_role".to_string());
            continue;
        };
        let path = string(row, "agent_manifest_path");
        if path != *expected_path {
            out.push(format!("plugin_self_law_registry_agent_mismatch:{role}"));
        }
        if !paths.insert(path) {
            out.push("plugin_self_law_registry_agent_duplicate_path".to_string());
        }
    }
    for (role, path) in reviewers::REQUIRED_REVIEWERS {
        let matches = rows
            .iter()
            .filter(|row| string(row, "role") == *role)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            out.push(format!("plugin_self_law_registry_agent_missing:{role}"));
            continue;
        }
        reviewer_row_failures(matches[0], role, path, &mut out);
    }
    out.push("plugin_self_law_registry_live_exposure_unavailable".to_string());
    out
}

fn reviewer_row_failures(row: &Value, role: &str, path: &str, out: &mut Vec<String>) {
    if string(row, "agent_manifest_path") != path {
        out.push(format!("plugin_self_law_registry_agent_mismatch:{role}"));
    }
    if string(row, "runtime_metadata_status") != "unavailable"
        || string(row, "custom_agent_discovery_status") != "unavailable"
    {
        out.push("plugin_self_law_registry_agent_runtime_unverified".to_string());
    }
    if row.get("exposed").and_then(Value::as_bool) != Some(false) {
        out.push("plugin_self_law_registry_agent_exposure_claim_invalid".to_string());
    }
    if string(row, "sandbox_mode") != "read-only" {
        out.push("plugin_self_law_registry_agent_sandbox_not_read_only".to_string());
    }
}

fn reject_legacy_keys(row: &Value, out: &mut Vec<String>) {
    if ["agent_type", "persona", "custom_agent_path"]
        .iter()
        .any(|key| row.get(key).is_some())
    {
        out.push("plugin_self_law_registry_agent_legacy_keys".to_string());
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
