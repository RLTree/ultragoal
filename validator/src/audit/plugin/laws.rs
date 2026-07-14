use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::path::Path;

const COVERAGE_RECEIPT: &str = "validation_artifacts/coverage/coverage-receipt.json";

pub fn package_failures(root: &Path, store: &schema_catalog::SchemaStore) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(version_failures(root));
    out.extend(coverage_failures(root, store));
    out.extend(crate::audit::plugin::registry::claim_guard_failures(
        root, store,
    ));
    out.extend(line_cap_failures(root));
    out.extend(crate::audit::plugin::dependency_adapter::failures(root));
    out.extend(crate::audit::plugin::live_repository_routes::failures(root));
    out
}

fn version_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let manifest = read(root, "plugin-manifest-draft.json", &mut out);
    let plugin = read(root, ".codex-plugin/plugin.json", &mut out);
    if let (Some(manifest), Some(plugin)) = (manifest, plugin) {
        let manifest_version = string(&manifest, "version");
        let plugin_version = string(&plugin, "version");
        if manifest_version.is_empty() || plugin_version.is_empty() {
            out.push("plugin_self_law_version_missing".to_string());
        } else if manifest_version != plugin_version {
            out.push(format!(
                "plugin_self_law_version_mismatch:{manifest_version}!={plugin_version}"
            ));
        }
    }
    out
}

fn coverage_failures(root: &Path, store: &schema_catalog::SchemaStore) -> Vec<String> {
    let mut out = Vec::new();
    let Some(receipt) = read(root, COVERAGE_RECEIPT, &mut out) else {
        return out;
    };
    out.extend(
        schema_catalog::schema_errors(store, "coverage-receipt.schema.json", &receipt)
            .into_iter()
            .map(|err| format!("plugin_self_law_coverage_schema:{err}")),
    );
    if receipt.pointer("/coverage/policy").and_then(Value::as_str) != Some("100_percent_required") {
        out.push("plugin_self_law_coverage_policy_not_100_required".to_string());
    }
    if receipt.pointer("/coverage/percent").and_then(Value::as_f64) != Some(100.0) {
        out.push("plugin_self_law_coverage_not_100_percent".to_string());
    }
    if !array(&receipt, "uncovered_records").is_empty() {
        out.push("plugin_self_law_coverage_has_uncovered_records".to_string());
    }
    if string(&receipt, "claim_ceiling") != "supports_complete_coverage_claim" {
        out.push("plugin_self_law_coverage_claim_ceiling_not_complete".to_string());
    }
    if !array_contains(&receipt, "supported_claim_classes", "complete_coverage") {
        out.push("plugin_self_law_coverage_supported_claim_missing".to_string());
    }
    for blocked in [
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure",
    ] {
        if !array_contains(&receipt, "blocked_claim_classes", blocked) {
            out.push(format!(
                "plugin_self_law_coverage_missing_blocked_claim:{blocked}"
            ));
        }
    }
    if receipt
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        == Some("unavailable")
    {
        out.push("plugin_self_law_coverage_target_revision_unavailable".to_string());
    }
    out
}

fn line_cap_failures(root: &Path) -> Vec<String> {
    let audit = crate::audit::source_governance::audit(root);
    let mut failures = audit.failures;
    failures.extend(crate::audit::source_governance::line_cap_failures(
        &audit.inventory,
    ));
    failures
}

fn read(root: &Path, rel: &str, out: &mut Vec<String>) -> Option<Value> {
    match json_boundary::read_json(&root.join(rel)) {
        Ok(value) => Some(value),
        Err(err) => {
            out.push(format!(
                "plugin_self_law_json_missing_or_malformed:{rel}:{err}"
            ));
            None
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

fn array_contains(value: &Value, key: &str, needle: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(needle)))
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
