use serde_json::Value;
use std::path::Path;

const FIT_ENTRYPOINT: &str = "harness-ultragoal:fit-repo";

pub(crate) fn failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = base_failures(root, receipt);
    out.extend(classification_failures(receipt));
    out.extend(check_artifact_failures(root, receipt));
    out.extend(blocker_failures(receipt));
    out
}

pub(crate) fn canonical_digest(receipt: &Value) -> String {
    let mut canonical = receipt.clone();
    if let Some(obj) = canonical.as_object_mut() {
        obj.insert(
            "receipt_digest".to_string(),
            Value::String(crate::digest::ZERO.to_string()),
        );
    }
    crate::digest::bytes(&serde_json::to_vec(&canonical).unwrap_or_default())
}

fn base_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if str_field(receipt, "schema") != "harness-ultragoal.fit-repo-receipt.v1" {
        out.push("fit_repo_receipt_malformed:schema".to_string());
    }
    if receipt
        .pointer("/entrypoint_contract/id")
        .and_then(Value::as_str)
        != Some(FIT_ENTRYPOINT)
    {
        out.push("plugin_flow_entrypoint_missing".to_string());
    }
    match crate::package::inventory::package_digest(root) {
        Ok(current)
            if receipt
                .pointer("/target_revision/value")
                .and_then(Value::as_str)
                == Some(current.as_str()) => {}
        Ok(_) => out.push("fit_repo_receipt_target_digest_mismatch".to_string()),
        Err(err) => out.push(format!("fit_repo_receipt_target_digest_unavailable:{err}")),
    }
    for key in [
        "plugin_source_path",
        "installed_plugin_path",
        "cache_package_path",
    ] {
        if str_field(receipt, key).is_empty() {
            out.push(format!("fit_repo_receipt_surface_missing:{key}"));
        }
    }
    out.extend(version_failures(root, receipt));
    out.extend(authority_failures(receipt));
    out
}

fn version_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let Ok(plugin) = crate::json_boundary::read_json(&root.join(".codex-plugin/plugin.json"))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let version = str_field(&plugin, "version");
    if str_field(receipt, "plugin_version") != version {
        out.push("fit_repo_receipt_wrong_plugin_version".to_string());
    }
    let expected_cache = format!("local-harness-plugins/harness-ultragoal/{version}");
    if str_field(receipt, "cache_package_path") != expected_cache {
        out.push("fit_repo_receipt_wrong_cache_package".to_string());
    }
    out
}

fn authority_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if str_field(receipt, "producer_actor_id") == "fixture-author"
        || str_field(receipt, "producer_actor_id").is_empty()
    {
        out.push("fit_repo_receipt_placeholder_actor".to_string());
    }
    if str_field(receipt, "receipt_digest") == crate::digest::ZERO {
        out.push("fit_repo_receipt_placeholder_digest".to_string());
    }
    if str_field(receipt, "receipt_digest") != canonical_digest(receipt) {
        out.push("fit_repo_receipt_digest_mismatch".to_string());
    }
    out
}

fn classification_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if str_field(receipt, "target_classification").is_empty()
        || str_field(receipt, "target_classification") == "blocked_unclassified_repo"
    {
        out.push("fit_repo_receipt_unclassified_target".to_string());
    }
    if str_field(receipt, "runtime_surface_classification").is_empty() {
        out.push("fit_repo_receipt_unclassified_runtime_surface".to_string());
    }
    if str_field(receipt, "product_surface_classification").is_empty()
        || str_field(receipt, "product_surface_classification") == "ambiguous_requires_blocker"
    {
        out.push("fit_repo_receipt_unclassified_product_surface".to_string());
    }
    if receipt
        .get("checks")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push("fit_repo_receipt_missing_check_result".to_string());
    }
    if str_field(receipt, "claim_ceiling").is_empty() {
        out.push("fit_repo_receipt_claim_ceiling_missing".to_string());
    }
    out
}

fn check_artifact_failures(root: &Path, receipt: &Value) -> Vec<String> {
    receipt
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|check| {
            ["stdout", "stderr"]
                .into_iter()
                .filter_map(|key| check.get(key))
        })
        .filter_map(|artifact| artifact_failure(root, artifact))
        .collect()
}

fn artifact_failure(root: &Path, artifact: &Value) -> Option<String> {
    let path = str_field(artifact, "path");
    if !path.starts_with("validation_artifacts/harness/fit-repo-command.") {
        return Some("fit_repo_receipt_command_output_not_command_artifact".to_string());
    }
    crate::package::artifact::refs::validate_object(root, artifact, "fit-repo command artifact")
        .err()
        .map(|err| format!("fit_repo_receipt_artifact_invalid:{err}"))
}

fn blocker_failures(receipt: &Value) -> Vec<String> {
    receipt
        .get("blockers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|blocker| str_field(blocker, "owner").is_empty())
        .map(|_| "fit_repo_receipt_blocker_without_owner".to_string())
        .collect()
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
