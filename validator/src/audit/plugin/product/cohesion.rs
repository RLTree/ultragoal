use serde_json::Value;
use std::path::Path;

const FLOW: &str = "docs/plugin-cohesion-manifest.json";
const FIT_RECEIPT: &str = "validation_artifacts/harness/fit-repo-receipt.json";
const JOURNEY: &str = "validation_artifacts/harness/plugin-product-journey-receipt.json";
const FIT_ENTRYPOINT: &str = "harness-ultragoal:fit-repo";

pub fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for path in [
        "skills/fit-repo/SKILL.md",
        "schemas/fit-repo-receipt.schema.json",
        "schemas/plugin-cohesion-manifest.schema.json",
        FLOW,
        FIT_RECEIPT,
        JOURNEY,
    ] {
        if !root.join(path).is_file() {
            out.push(format!("plugin_product_surface_missing:{path}"));
        }
    }
    out.extend(flow_failures(root));
    out.extend(visible_entry_failures(root));
    out.extend(fit_receipt_failures(root));
    out.extend(journey_failures(root));
    out
}

fn flow_failures(root: &Path) -> Vec<String> {
    let flow = match crate::json_boundary::read_json(&root.join(FLOW)) {
        Ok(value) => value,
        Err(err) => return vec![format!("plugin_flow_manifest_malformed:{err}")],
    };
    flow_value_failures(root, &flow)
}

pub fn flow_value_failures(root: &Path, flow: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if str_field(&flow, "schema") != "harness-ultragoal.plugin-cohesion-manifest.v1" {
        out.push("plugin_flow_manifest_malformed:schema".to_string());
    }
    let entries = strings(&flow, "entrypoints");
    if !entries.iter().any(|entry| entry == FIT_ENTRYPOINT) {
        out.push("plugin_flow_entrypoint_missing".to_string());
    }
    if flow
        .get("edges")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push("plugin_flow_required_edge_missing".to_string());
    }
    out.extend(crate::audit::plugin::flow::authority::failures(&flow));
    for surface in strings(&flow, "required_surfaces") {
        if !root.join(&surface).is_file() {
            out.push(format!("plugin_flow_setup_file_not_packaged:{surface}"));
        }
    }
    out.extend(category_failures(root, &flow));
    out
}

fn category_failures(root: &Path, flow: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .unwrap_or(Value::Null);
    require_manifest_paths(&manifest, flow, "skills", "skills", &mut out);
    require_manifest_paths(&manifest, flow, "schemas", "schemas", &mut out);
    require_manifest_paths(
        &manifest,
        flow,
        "authorable_templates",
        "templates",
        &mut out,
    );
    require_manifest_paths(&manifest, flow, "agents", "custom_agents", &mut out);
    for key in [
        "setup_scripts",
        "fixture_groups",
        "receipts",
        "validator_checks",
        "standards_rows",
        "package_cache_install_surfaces",
    ] {
        if strings(flow, key).is_empty() {
            out.push(format!("plugin_flow_category_missing:{key}"));
        }
    }
    let standards =
        crate::json_boundary::read_json(&root.join("templates/agent-standards/enforcement.json"))
            .unwrap_or(Value::Null);
    let declared = strings(flow, "standards_rows");
    for row in standards
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
    {
        if !declared.iter().any(|item| item == row) {
            out.push(format!("plugin_flow_standards_row_missing:{row}"));
        }
    }
    out
}

fn require_manifest_paths(
    manifest: &Value,
    flow: &Value,
    manifest_key: &str,
    flow_key: &str,
    out: &mut Vec<String>,
) {
    let declared = strings(flow, flow_key);
    let paths = manifest
        .get(manifest_key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.as_str().map(ToOwned::to_owned).or_else(|| {
                row.get("path")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
        });
    for path in paths {
        if !declared.iter().any(|item| item == &path) {
            out.push(format!(
                "plugin_flow_manifest_path_missing:{flow_key}:{path}"
            ));
        }
    }
}

fn visible_entry_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let plugin =
        std::fs::read_to_string(root.join(".codex-plugin/plugin.json")).unwrap_or_default();
    if !plugin.contains(FIT_ENTRYPOINT) {
        out.push("plugin_flow_entrypoint_not_visible".to_string());
    }
    let map = std::fs::read_to_string(root.join("docs/plugin-resource-map.md")).unwrap_or_default();
    let fit = map.find(FIT_ENTRYPOINT).unwrap_or(usize::MAX);
    let legacy = [
        "`ultragoal`",
        "`harness-engineering`",
        "`agent-first-repo-init`",
    ]
    .iter()
    .filter_map(|needle| map.find(needle))
    .min()
    .unwrap_or(usize::MAX);
    if fit == usize::MAX || legacy < fit {
        out.push("plugin_flow_entrypoint_not_primary".to_string());
    }
    out
}

pub fn plugin_json_failures(value: &Value) -> Vec<String> {
    let text = value
        .get("interface")
        .and_then(|item| item.get("defaultPrompt"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    if text.contains(FIT_ENTRYPOINT) {
        Vec::new()
    } else {
        vec!["plugin_flow_entrypoint_not_visible".to_string()]
    }
}

fn fit_receipt_failures(root: &Path) -> Vec<String> {
    let receipt = match crate::json_boundary::read_json(&root.join(FIT_RECEIPT)) {
        Ok(value) => value,
        Err(err) => return vec![format!("fit_repo_receipt_missing:{err}")],
    };
    fit_receipt_value_failures(root, &receipt)
}

pub fn fit_receipt_value_failures(root: &Path, receipt: &Value) -> Vec<String> {
    crate::audit::fit_repo_receipt::failures(root, receipt)
}

pub fn fit_receipt_value_failures_with_candidate(
    root: &Path,
    receipt: &Value,
    target_digest: &str,
) -> Vec<String> {
    crate::audit::fit_repo_receipt::failures_with_candidate(root, receipt, target_digest)
}

fn journey_failures(root: &Path) -> Vec<String> {
    let value = match crate::json_boundary::read_json(&root.join(JOURNEY)) {
        Ok(value) => value,
        Err(err) => return vec![format!("plugin_product_journey_missing:{err}")],
    };
    journey_value_failures(root, &value)
}

pub fn journey_value_failures(root: &Path, value: &Value) -> Vec<String> {
    crate::audit::plugin::product::journey::failures(root, value)
}

pub fn journey_value_failures_with_candidate(
    root: &Path,
    value: &Value,
    target_digest: &str,
) -> Vec<String> {
    crate::audit::plugin::product::journey::failures_with_candidate(root, value, target_digest)
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
