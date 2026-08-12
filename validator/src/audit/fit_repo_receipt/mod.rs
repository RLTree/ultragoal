use serde_json::Value;
use std::path::Path;

mod record;

use record::{FIT_REPO_ENTRYPOINT, FitRepoReceipt};

pub(crate) fn failures(root: &Path, receipt: &Value) -> Vec<String> {
    let parsed = FitRepoReceipt::from_value(receipt);
    let mut out = base_failures(root, &parsed);
    out.extend(classification_failures(&parsed));
    out.extend(check_artifact_failures(root, &parsed));
    out.extend(blocker_failures(&parsed));
    out.extend(digest_failures(receipt, &parsed));
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

fn base_failures(root: &Path, receipt: &FitRepoReceipt) -> Vec<String> {
    base_failures_with_candidate(
        root,
        receipt,
        crate::package::inventory::package_digest(root),
    )
}

fn base_failures_with_candidate(
    root: &Path,
    receipt: &FitRepoReceipt,
    current: Result<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.schema != "harness-ultragoal.fit-repo-receipt.v1" {
        out.push("fit_repo_receipt_malformed:schema".to_string());
    }
    if receipt.entrypoint_id != FIT_REPO_ENTRYPOINT {
        out.push("plugin_flow_entrypoint_missing".to_string());
    }
    match current {
        Ok(current) if receipt.target_revision_value == current => {}
        Ok(_) => out.push("fit_repo_receipt_target_digest_mismatch".to_string()),
        Err(err) => out.push(format!("fit_repo_receipt_target_digest_unavailable:{err}")),
    }
    if receipt.plugin_source_path.is_empty() {
        out.push("fit_repo_receipt_surface_missing:plugin_source_path".to_string());
    }
    if receipt.installed_plugin_path.is_empty() {
        out.push("fit_repo_receipt_surface_missing:installed_plugin_path".to_string());
    }
    if receipt.cache_package_path.is_empty() {
        out.push("fit_repo_receipt_surface_missing:cache_package_path".to_string());
    }
    out.extend(version_failures(root, receipt));
    out.extend(authority_failures(receipt));
    out
}

fn version_failures(root: &Path, receipt: &FitRepoReceipt) -> Vec<String> {
    let Ok(plugin) = crate::json_boundary::read_json(&root.join(".codex-plugin/plugin.json"))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let version = str_field(&plugin, "version");
    if receipt.plugin_version != version {
        out.push("fit_repo_receipt_wrong_plugin_version".to_string());
    }
    let expected_cache = format!("local-harness-plugins/harness-ultragoal/{version}");
    if receipt.cache_package_path != expected_cache {
        out.push("fit_repo_receipt_wrong_cache_package".to_string());
    }
    out
}

fn authority_failures(receipt: &FitRepoReceipt) -> Vec<String> {
    let mut out = vec!["fit_repo_receipt_independent_authority_unavailable".to_string()];
    if receipt.producer_actor_id == "fixture-author" || receipt.producer_actor_id.is_empty() {
        out.push("fit_repo_receipt_placeholder_actor".to_string());
    }
    if producer_actor_lacks_product_authority(&receipt.producer_actor_id) {
        out.push("fit_repo_receipt_unowned_producer_actor".to_string());
    }
    if receipt.receipt_digest == crate::digest::ZERO {
        out.push("fit_repo_receipt_placeholder_digest".to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn arbitrary_actor_and_self_hash_cannot_mint_fit_authority() {
        let receipt = super::FitRepoReceipt::from_value(&json!({
            "producer_actor_id": "plausible-production-actor",
            "receipt_digest": crate::self_tests::boundaries::workspace_fixtures::sha('a')
        }));
        let failures = super::authority_failures(&receipt);
        assert!(
            failures.contains(&"fit_repo_receipt_independent_authority_unavailable".to_string()),
            "{failures:?}"
        );
    }
}

fn producer_actor_lacks_product_authority(actor: &str) -> bool {
    let actor = actor.to_ascii_lowercase();
    let parent_session = concat!("parent", "-", "session");
    let history_session = concat!("session", "-", "history");
    actor.contains(parent_session)
        || actor.contains(history_session)
        || actor.starts_with(concat!("session", "-"))
        || actor.starts_with(concat!("session", "_"))
}

fn digest_failures(raw: &Value, receipt: &FitRepoReceipt) -> Vec<String> {
    if receipt.receipt_digest == canonical_digest(raw) {
        Vec::new()
    } else {
        vec!["fit_repo_receipt_digest_mismatch".to_string()]
    }
}

fn classification_failures(receipt: &FitRepoReceipt) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.target_classification.is_empty()
        || receipt.target_classification == "blocked_unclassified_repo"
    {
        out.push("fit_repo_receipt_unclassified_target".to_string());
    }
    if receipt.runtime_surface_classification.is_empty() {
        out.push("fit_repo_receipt_unclassified_runtime_surface".to_string());
    }
    if receipt.product_surface_classification.is_empty()
        || receipt.product_surface_classification == "ambiguous_requires_blocker"
    {
        out.push("fit_repo_receipt_unclassified_product_surface".to_string());
    }
    if receipt.check_count == 0 {
        out.push("fit_repo_receipt_missing_check_result".to_string());
    }
    if receipt.claim_ceiling.is_empty() {
        out.push("fit_repo_receipt_claim_ceiling_missing".to_string());
    }
    out
}

fn check_artifact_failures(root: &Path, receipt: &FitRepoReceipt) -> Vec<String> {
    receipt
        .check_artifacts
        .iter()
        .filter_map(|artifact| artifact_failure(root, artifact))
        .collect()
}

fn artifact_failure(
    root: &Path,
    artifact: &Result<crate::package::artifact::refs::ArtifactRef, String>,
) -> Option<String> {
    let artifact = match artifact {
        Ok(artifact) => artifact,
        Err(err) => return Some(format!("fit_repo_receipt_artifact_invalid:{err}")),
    };
    if !artifact
        .path()
        .starts_with("validation_artifacts/harness/fit-repo-command.")
    {
        return Some("fit_repo_receipt_command_output_not_command_artifact".to_string());
    }
    artifact
        .validate(root, "fit-repo command artifact")
        .err()
        .map(|err| format!("fit_repo_receipt_artifact_invalid:{err}"))
}

fn blocker_failures(receipt: &FitRepoReceipt) -> Vec<String> {
    receipt
        .blocker_owners
        .iter()
        .filter(|owner| owner.is_empty())
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
