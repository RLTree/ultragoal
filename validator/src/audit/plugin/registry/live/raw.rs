use serde_json::Value;
use std::path::Path;

pub(super) fn observation_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let Some(raw) = receipt.get("raw_observation") else {
        return vec!["plugin_self_law_registry_raw_observation_missing".to_string()];
    };
    let path = string(raw, "path");
    if !path.starts_with("validation_artifacts/ultragoal-audit/")
        || crate::package::inventory::package_path_error(root, path).is_some()
    {
        return vec![format!(
            "plugin_self_law_registry_raw_observation_path_invalid:{path}"
        )];
    }
    let expected = string(raw, "digest");
    let mut out = match crate::digest::file(&root.join(path)) {
        Ok(actual) if actual == expected => Vec::new(),
        _ => vec![format!(
            "plugin_self_law_registry_raw_observation_digest_mismatch:{path}"
        )],
    };
    if receipt.get("status").and_then(Value::as_str) == Some("pass") {
        match crate::json_boundary::read_json(&root.join(path)) {
            Ok(value) => out.extend(live_tool_failures(&value, receipt)),
            Err(err) => out.push(format!(
                "plugin_self_law_registry_raw_observation_malformed:{path}:{err}"
            )),
        }
    }
    out
}

fn live_tool_failures(raw: &Value, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if string(raw, "schema") != "harness-ultragoal.registry-raw-observation.v1" {
        out.push("plugin_self_law_registry_raw_observation_wrong_schema".to_string());
    }
    out.extend(pointer_pair_failures(raw, receipt));
    if ptr_string(raw, "/tool_call/name") != "multi_agent_v1.tool_registry" {
        out.push("plugin_self_law_registry_raw_observation_not_tool_registry".to_string());
    }
    out.extend(reviewer_row_failures(raw));
    out
}

fn pointer_pair_failures(raw: &Value, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for (raw_ptr, receipt_ptr, code) in POINTER_PAIRS {
        if ptr_string(raw, raw_ptr) != ptr_string(receipt, receipt_ptr) {
            out.push(format!("plugin_self_law_registry_raw_observation_{code}"));
        }
    }
    out
}

const POINTER_PAIRS: &[(&str, &str, &str)] = &[
    (
        "/candidate_digest",
        "/target_revision/value",
        "candidate_digest_mismatch",
    ),
    ("/captured_at", "/captured_at", "captured_at_mismatch"),
    ("/issuer/tool", "/issuer/tool", "issuer_tool_mismatch"),
    (
        "/issuer/authority",
        "/issuer/authority",
        "issuer_authority_mismatch",
    ),
    (
        "/tool_call/name",
        "/tool_call/name",
        "tool_call_name_mismatch",
    ),
    (
        "/tool_call/call_id",
        "/tool_call/call_id",
        "tool_call_call_id_mismatch",
    ),
    (
        "/tool_call/arguments_digest",
        "/tool_call/arguments_digest",
        "tool_call_arguments_digest_mismatch",
    ),
    (
        "/boundary/account_id",
        "/boundary/account_id",
        "boundary_account_mismatch",
    ),
    (
        "/boundary/workspace_id",
        "/boundary/workspace_id",
        "boundary_workspace_mismatch",
    ),
    (
        "/boundary/session_id",
        "/boundary/session_id",
        "boundary_session_mismatch",
    ),
    ("/source", "/source", "source_mismatch"),
];

fn reviewer_row_failures(raw: &Value) -> Vec<String> {
    let rows = raw
        .get("registry_rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for (agent_type, _, _) in super::reviewers::REQUIRED_REVIEWERS {
        let matches = rows
            .iter()
            .filter(|row| string(row, "agent_type") == *agent_type)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            out.push(format!(
                "plugin_self_law_registry_raw_observation_agent_missing:{agent_type}"
            ));
            continue;
        }
        if matches[0].get("exposed").and_then(Value::as_bool) != Some(true) {
            out.push(format!(
                "plugin_self_law_registry_raw_observation_agent_not_exposed:{agent_type}"
            ));
        }
    }
    out
}

fn ptr_string<'a>(value: &'a Value, ptr: &str) -> &'a str {
    value.pointer(ptr).and_then(Value::as_str).unwrap_or("")
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
