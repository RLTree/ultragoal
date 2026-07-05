use serde_json::Value;
use std::path::Path;

pub(crate) mod manifest;

use manifest::{
    PackageManifestProjection, PluginCohesionManifest, PluginPromptProjection,
    PluginResourceMapProjection, StandardsRowProjection,
};

const FLOW: &str = "docs/plugin-cohesion-manifest.json";
const FIT_REPO_RECEIPT: &str = "validation_artifacts/harness/fit-repo-receipt.json";
const JOURNEY: &str = "validation_artifacts/harness/plugin-product-journey-receipt.json";
const FIT_REPO_ENTRYPOINT: &str = "harness-ultragoal:fit-repo";

pub fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for path in [
        "skills/fit-repo/SKILL.md",
        "schemas/fit-repo-receipt.schema.json",
        "schemas/plugin-cohesion-manifest.schema.json",
        FLOW,
        FIT_REPO_RECEIPT,
        JOURNEY,
    ] {
        if !root.join(path).is_file() {
            out.push(format!("plugin_product_surface_missing:{path}"));
        }
    }
    out.extend(flow_failures(root));
    out.extend(visible_entry_failures(root));
    out.extend(fit_repo_receipt_failures(root));
    out.extend(journey_failures(root));
    out
}

fn flow_failures(root: &Path) -> Vec<String> {
    let value = match crate::json_boundary::read_json(&root.join(FLOW)) {
        Ok(value) => value,
        Err(err) => return vec![format!("plugin_flow_manifest_malformed:{err}")],
    };
    flow_manifest_failures(root, &PluginCohesionManifest::from_value(&value))
}

pub(crate) fn flow_manifest_projection_failures(root: &Path, flow: &Value) -> Vec<String> {
    flow_manifest_failures(root, &PluginCohesionManifest::from_value(flow))
}

fn flow_manifest_failures(root: &Path, flow: &PluginCohesionManifest) -> Vec<String> {
    let mut out = Vec::new();
    if flow.schema != "harness-ultragoal.plugin-cohesion-manifest.v1" {
        out.push("plugin_flow_manifest_malformed:schema".to_string());
    }
    if !flow
        .entrypoints
        .iter()
        .any(|entry| entry == FIT_REPO_ENTRYPOINT)
    {
        out.push("plugin_flow_entrypoint_missing".to_string());
    }
    if flow.edges.is_empty() {
        out.push("plugin_flow_required_edge_missing".to_string());
    }
    out.extend(crate::audit::plugin::flow::authority::failures(flow));
    for surface in &flow.required_surfaces {
        if !root.join(&surface).is_file() {
            out.push(format!("plugin_flow_setup_file_not_packaged:{surface}"));
        }
    }
    out.extend(category_failures(root, flow));
    out
}

fn category_failures(root: &Path, flow: &PluginCohesionManifest) -> Vec<String> {
    let mut out = Vec::new();
    let manifest = PackageManifestProjection::from_value(
        &crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
            .unwrap_or(Value::Null),
    );
    require_manifest_paths(&manifest.skills, &flow.skills, "skills", &mut out);
    require_manifest_paths(&manifest.schemas, &flow.schemas, "schemas", &mut out);
    require_manifest_paths(
        &manifest.authorable_templates,
        &flow.templates,
        "templates",
        &mut out,
    );
    require_manifest_paths(
        &manifest.agents,
        &flow.custom_agents,
        "custom_agents",
        &mut out,
    );
    for (key, values) in [
        ("setup_scripts", &flow.setup_scripts),
        ("fixture_groups", &flow.fixture_groups),
        ("receipts", &flow.receipts),
        ("validator_checks", &flow.validator_checks),
        ("standards_rows", &flow.standards_rows),
        (
            "package_cache_install_surfaces",
            &flow.package_cache_install_surfaces,
        ),
    ] {
        if values.is_empty() {
            out.push(format!("plugin_flow_category_missing:{key}"));
        }
    }
    let standards = StandardsRowProjection::from_value(
        &crate::json_boundary::read_json(&root.join("templates/agent-standards/enforcement.json"))
            .unwrap_or(Value::Null),
    );
    for row in standards.row_ids {
        if !flow.standards_rows.iter().any(|item| item == &row) {
            out.push(format!("plugin_flow_standards_row_missing:{row}"));
        }
    }
    out
}

fn require_manifest_paths(
    manifest_paths: &[String],
    declared: &[String],
    flow_key: &str,
    out: &mut Vec<String>,
) {
    for path in manifest_paths {
        if !declared.iter().any(|item| item == path) {
            out.push(format!(
                "plugin_flow_manifest_path_missing:{flow_key}:{path}"
            ));
        }
    }
}

fn visible_entry_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let plugin = PluginPromptProjection::from_value(
        &crate::json_boundary::read_json(&root.join(".codex-plugin/plugin.json"))
            .unwrap_or(Value::Null),
    );
    if !plugin.mentions(FIT_REPO_ENTRYPOINT) {
        out.push("plugin_flow_entrypoint_not_visible".to_string());
    }
    let map = PluginResourceMapProjection::from_text(
        std::fs::read_to_string(root.join("docs/plugin-resource-map.md")).unwrap_or_default(),
    );
    let fit_repo_entry = map
        .first_position(FIT_REPO_ENTRYPOINT)
        .unwrap_or(usize::MAX);
    let legacy = [
        "`ultragoal`",
        "`harness-engineering`",
        "`agent-first-repo-init`",
    ]
    .iter()
    .filter_map(|needle| map.first_position(needle))
    .min()
    .unwrap_or(usize::MAX);
    if fit_repo_entry == usize::MAX || legacy < fit_repo_entry {
        out.push("plugin_flow_entrypoint_not_primary".to_string());
    }
    out
}

pub fn plugin_json_failures(value: &Value) -> Vec<String> {
    let prompt = PluginPromptProjection::from_value(value);
    if prompt.mentions(FIT_REPO_ENTRYPOINT) {
        Vec::new()
    } else {
        vec!["plugin_flow_entrypoint_not_visible".to_string()]
    }
}

fn fit_repo_receipt_failures(root: &Path) -> Vec<String> {
    let receipt = match crate::json_boundary::read_json(&root.join(FIT_REPO_RECEIPT)) {
        Ok(value) => value,
        Err(err) => return vec![format!("fit_repo_receipt_missing:{err}")],
    };
    fit_repo_receipt_value_failures(root, &receipt)
}

pub fn fit_repo_receipt_value_failures(root: &Path, receipt: &Value) -> Vec<String> {
    crate::audit::fit_repo_receipt::failures(root, receipt)
}

pub fn fit_repo_receipt_value_failures_with_candidate(
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
