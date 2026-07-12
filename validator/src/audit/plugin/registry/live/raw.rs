use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Component, Path};

const RAW_PREFIX: &str = "validation_artifacts/ultragoal-audit/";
const MAX_RAW_BYTES: u64 = 1024 * 1024;

struct RawArtifact {
    bytes: Vec<u8>,
    value: Value,
}

pub(super) fn observation_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let Some(raw) = receipt.get("raw_observation") else {
        return vec!["plugin_self_law_registry_raw_observation_missing".to_string()];
    };
    let path = string(raw, "path");
    if !allowed_path(path) {
        return vec!["plugin_self_law_registry_raw_observation_path_invalid".to_string()];
    }
    let artifact = match read_artifact(root, path) {
        Ok(artifact) => artifact,
        Err(code) => return vec![code.to_string()],
    };
    let expected = string(raw, "digest");
    let mut out = match crate::digest::bytes(&artifact.bytes) == expected {
        true => Vec::new(),
        false => vec!["plugin_self_law_registry_raw_observation_digest_mismatch".to_string()],
    };
    match receipt.get("status").and_then(Value::as_str) {
        Some("fail") => out.extend(fail_closed_failures(&artifact.value, receipt)),
        _ => {
            out.push(
                "plugin_self_law_registry_raw_observation_positive_proof_forbidden".to_string(),
            );
            out.extend(live_tool_failures(&artifact.value, receipt));
        }
    }
    out
}

fn allowed_path(path: &str) -> bool {
    path.starts_with(RAW_PREFIX)
        && path.ends_with(".json")
        && path.len() <= 4096
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn read_artifact(root: &Path, path: &str) -> Result<RawArtifact, &'static str> {
    let mut session = crate::package::inventory::anchored::Session::open(root)
        .map_err(|_| "plugin_self_law_registry_raw_observation_root_unavailable")?;
    let bytes = session
        .read(path, MAX_RAW_BYTES)
        .map_err(|_| "plugin_self_law_registry_raw_observation_unavailable")?;
    let value = crate::package::inventory::anchored::parse_unique_json(&bytes)
        .map_err(|_| "plugin_self_law_registry_raw_observation_malformed");
    session
        .finish()
        .map_err(|_| "plugin_self_law_registry_raw_observation_changed_during_read")?;
    Ok(RawArtifact {
        bytes,
        value: value?,
    })
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

fn fail_closed_failures(raw: &Value, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if string(raw, "schema") != "harness-ultragoal.registry-observation.v1"
        || string(raw, "status") != "fail"
    {
        out.push("plugin_self_law_registry_raw_observation_fail_closed_shape".to_string());
    }
    if string(raw, "candidate_digest")
        != receipt
            .pointer("/target_revision/value")
            .and_then(Value::as_str)
            .unwrap_or("")
    {
        out.push("plugin_self_law_registry_raw_observation_candidate_digest_mismatch".to_string());
    }
    if string(raw, "observed").is_empty()
        || raw.pointer("/probe/source").and_then(Value::as_str) != Some("ultragoal.registry_probe")
        || raw.pointer("/probe/command").and_then(Value::as_str) != Some("ultragoal registry probe")
        || raw.pointer("/probe/capture_method").and_then(Value::as_str)
            != Some("fail_closed_no_capability")
    {
        out.push("plugin_self_law_registry_raw_observation_fail_closed_provenance".to_string());
    }
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
    if rows.len() != super::reviewers::REQUIRED_REVIEWERS.len() {
        out.push("plugin_self_law_registry_raw_observation_agent_count_mismatch".to_string());
    }
    let mut paths = BTreeSet::new();
    for row in &rows {
        if ["agent_type", "persona", "custom_agent_path"]
            .iter()
            .any(|key| row.get(key).is_some())
        {
            out.push("plugin_self_law_registry_raw_observation_agent_legacy_keys".to_string());
        }
        let role = string(row, "role");
        let Some((_, expected_path)) = super::reviewers::REQUIRED_REVIEWERS
            .iter()
            .find(|(expected_role, _)| *expected_role == role)
        else {
            out.push("plugin_self_law_registry_raw_observation_agent_unexpected_role".to_string());
            continue;
        };
        let path = string(row, "agent_manifest_path");
        if path != *expected_path {
            out.push("plugin_self_law_registry_raw_observation_agent_mismatch".to_string());
        }
        if !paths.insert(path) {
            out.push("plugin_self_law_registry_raw_observation_agent_duplicate_path".to_string());
        }
    }
    for (role, path) in super::reviewers::REQUIRED_REVIEWERS {
        let matches = rows
            .iter()
            .filter(|row| string(row, "role") == *role)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            out.push(format!(
                "plugin_self_law_registry_raw_observation_agent_missing:{role}"
            ));
            continue;
        }
        if string(&matches[0], "agent_manifest_path") != *path {
            out.push("plugin_self_law_registry_raw_observation_agent_mismatch".to_string());
        }
        if string(&matches[0], "runtime_metadata_status") != "unavailable"
            || string(&matches[0], "custom_agent_discovery_status") != "unavailable"
        {
            out.push(
                "plugin_self_law_registry_raw_observation_agent_runtime_unverified".to_string(),
            );
        }
        if matches[0].get("exposed").and_then(Value::as_bool) != Some(false) {
            out.push(
                "plugin_self_law_registry_raw_observation_agent_exposure_claim_invalid".to_string(),
            );
        }
        if string(&matches[0], "sandbox_mode") != "read-only" {
            out.push(
                "plugin_self_law_registry_raw_observation_agent_sandbox_not_read_only".to_string(),
            );
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
