use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::path::Path;

const COVERAGE_RECEIPT: &str = "validation_artifacts/coverage/coverage-receipt.json";
const MAX_SOURCE_LINES: usize = 250;

pub fn package_failures(root: &Path, store: &schema_catalog::SchemaStore) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(version_failures(root));
    out.extend(coverage_failures(root, store));
    out.extend(crate::audit::plugin::registry::claim_guard_failures(
        root, store,
    ));
    out.extend(line_cap_failures(root));
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
    if string(&receipt, "claim_ceiling") != "supports_complete_claim" {
        out.push("plugin_self_law_coverage_claim_ceiling_not_complete".to_string());
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
    let mut out = Vec::new();
    for rel in source_paths(root) {
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            out.push(format!("plugin_self_law_line_cap_unreadable:{rel}"));
            continue;
        };
        let lines = text.lines().count();
        if lines > MAX_SOURCE_LINES {
            out.push(format!("plugin_self_law_line_cap_exceeded:{rel}:{lines}"));
        }
    }
    out
}

fn source_paths(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    collect(root, "validator/src", ".rs", &mut out);
    collect(root, ".harness", ".sh", &mut out);
    for rel in [
        ".harness/coverage-command",
        "scripts/check",
        "scripts/check-agent-standards",
        "scripts/check-coverage-fast",
        "scripts/check-coverage-full",
    ] {
        if root.join(rel).is_file() {
            out.push(rel.to_string());
        }
    }
    out.sort();
    out
}

fn collect(root: &Path, rel: &str, suffix: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(root.join(rel)) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(child) = path.strip_prefix(root) {
                collect(
                    root,
                    &child.to_string_lossy().replace('\\', "/"),
                    suffix,
                    out,
                );
            }
        } else if path.to_string_lossy().ends_with(suffix)
            && let Ok(child) = path.strip_prefix(root)
        {
            out.push(child.to_string_lossy().replace('\\', "/"));
        }
    }
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

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
