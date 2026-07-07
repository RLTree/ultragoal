pub(crate) mod path_rules;
use crate::audit::namespace::law::path_rules::{mixed_domain_folder, root_route_allowed};
use crate::json_boundary;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const CLASS_REGISTRY_PATH: &str = "docs/namespace-class-registry.json";
const STANDARD_ID: &str = "namespace-progressive-disclosure";
const TRACE_OBLIGATION_ID: &str = "namespace-progressive-disclosure";

#[derive(Default)]
pub(crate) struct ValueCache {
    actual_files: Option<Vec<String>>,
    repo_source_paths: Option<Vec<String>>,
}

impl ValueCache {
    fn actual_files(&mut self, root: &Path) -> Vec<String> {
        self.actual_files
            .get_or_insert_with(|| {
                crate::package::inventory::closure::actual_files(root).unwrap_or_default()
            })
            .clone()
    }

    fn repo_source_paths(&mut self, root: &Path) -> Vec<String> {
        if let Some(paths) = &self.repo_source_paths {
            return paths.clone();
        }
        let actual_files = self.actual_files(root);
        let paths = crate::audit::namespace::source::topology::repo_source_paths_from_actual_files(
            &actual_files,
        );
        self.repo_source_paths = Some(paths.clone());
        paths
    }
}

pub fn package_failures(root: &Path, manifest: &Value) -> Vec<String> {
    let registry = match json_boundary::read_json(&root.join(CLASS_REGISTRY_PATH)) {
        Ok(value) => value,
        Err(err) => return vec![format!("namespace_class_registry_file_missing:{err}")],
    };
    let mut out = value_failures(root, manifest);
    out.extend(class_registry_value_failures(root, &registry));
    out.extend(crate::audit::namespace::classes::resolution_failures(
        &registry,
        &crate::package::inventory::inventory_paths(manifest),
    ));
    out.extend(binding_failures(root));
    out
}

pub fn value_failures(root: &Path, manifest: &Value) -> Vec<String> {
    let mut cache = ValueCache::default();
    value_failures_with_cache(root, manifest, &mut cache)
}

pub(crate) fn value_failures_with_cache(
    root: &Path,
    manifest: &Value,
    cache: &mut ValueCache,
) -> Vec<String> {
    let listed = crate::package::inventory::inventory_paths(manifest);
    let mut out = Vec::new();
    out.extend(path_name_failures(&listed));
    out.extend(
        crate::audit::namespace::source::topology::failures_with_repo_paths(
            &listed,
            &cache.repo_source_paths(root),
        ),
    );
    out.extend(crate::audit::namespace::source::identifiers::failures(
        root,
        &cache.actual_files(root),
    ));
    out.extend(crate::audit::namespace::classes::legacy_surface_failures(
        root, &listed,
    ));
    out.extend(orphan_file_failures_for_actual_files(
        &cache.actual_files(root),
        &listed,
    ));
    out
}

pub fn class_registry_value_failures(root: &Path, value: &Value) -> Vec<String> {
    crate::audit::namespace::classes::value_failures(root, value)
}

fn path_name_failures(listed: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for rel in listed {
        let components = rel.split('/').collect::<Vec<_>>();
        if let Some(label) =
            crate::audit::namespace::source::path_labels::product_opaque_goal_work_label(rel)
        {
            out.push(format!(
                "namespace_product_opaque_goal_work_path:path={rel};label={label};why=package_or_artifact_path_names_goal_work_or_evidence_posture_instead_of_product_behavior;repair=rename_package_or_artifact_path_by_product_behavior;claims=completion,review,package,readiness,release,product_readiness,cli_self_law,source_audit,final_packet,update_goal;exception_allowed=false"
            ));
        }
        if let Some(label) = schema_authority_leaf_label(rel, &components) {
            out.push(format!(
                "namespace_schema_generic_authority_path:path={rel};label={label};why=schema_authority_file_name_uses_generic_bucket_instead_of_schema_product_role;repair=rename_schema_file_by_authority_role_or_schema_domain;claims=completion,review,package,readiness,release,product_readiness,cli_self_law,source_audit,final_packet,update_goal;exception_allowed=false"
            ));
        }
        if components
            .iter()
            .any(|part| matches!(*part, "utils" | "helpers" | "misc"))
        {
            out.push(format!("namespace_junk_drawer_path:{rel}"));
        }
        if components.iter().any(|part| *part == "common") {
            out.push(format!("namespace_vague_common_domain_path:{rel}"));
        }
        if components.iter().any(|part| *part == "shared") {
            out.push(format!("namespace_vague_shared_domain_path:{rel}"));
        }
        if components.iter().any(|part| *part == "lib") {
            out.push(format!("namespace_vague_lib_domain_path:{rel}"));
        }
        if components.iter().any(|part| *part == "services") {
            out.push(format!("namespace_generic_services_path:{rel}"));
        }
        if components.len() > 8 {
            out.push(format!(
                "namespace_excessive_depth:{}:{rel}",
                components.len()
            ));
        }
        if mixed_domain_folder(&components[..components.len().saturating_sub(1)]) {
            out.push(format!("namespace_mixed_domain_folder:{rel}"));
        }
        if components.len() == 1
            && !root_route_allowed(rel)
            && rel.split('.').next().unwrap_or("").contains('-')
        {
            out.push(format!("namespace_root_clutter_without_route:{rel}"));
        }
    }
    out
}

fn schema_authority_leaf_label(rel: &str, components: &[&str]) -> Option<&'static str> {
    if !rel.starts_with("schemas/") {
        return None;
    }
    let leaf = components
        .last()
        .expect("split path components are never empty");
    let stem = leaf
        .strip_suffix(".schema.json")
        .or_else(|| leaf.strip_suffix(".json"))
        .unwrap_or(leaf);
    crate::audit::namespace::source::path_labels::generic_source_leaf_label(stem)
}

fn orphan_file_failures_for_actual_files(
    actual_files: &[String],
    listed: &[String],
) -> Vec<String> {
    let listed = listed.iter().cloned().collect::<BTreeSet<_>>();
    let unlisted = actual_files
        .iter()
        .filter(|rel| !listed.contains(rel.as_str()))
        .filter(|rel| !rel.starts_with("validation_artifacts/"))
        .filter(|rel| !generated_build_output(rel))
        .cloned()
        .collect::<Vec<_>>();
    match unlisted.len() {
        0 => Vec::new(),
        1..=20 => unlisted
            .into_iter()
            .map(|rel| format!("namespace_orphan_repo_file:{rel}"))
            .collect(),
        count => {
            let sample = unlisted.into_iter().take(20).collect::<Vec<_>>().join(",");
            vec![format!(
                "namespace_orphan_repo_file_count:{count}:sample:{sample}"
            )]
        }
    }
}

fn generated_build_output(rel: &str) -> bool {
    rel.starts_with("target/") || rel.starts_with("validator/target/")
}

pub(crate) fn binding_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if !standards_row_exists(root) {
        out.push("namespace_law_missing_standards_row".to_string());
    }
    if !trace_entry_exists(root) {
        out.push("namespace_law_missing_foundational_trace".to_string());
    }
    let red = json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    if !red.iter().any(|row| {
        row.get("expected_failure")
            .and_then(|failure| failure.get("check_id"))
            .and_then(Value::as_str)
            == Some("namespace-progressive-disclosure")
    }) {
        out.push("namespace_law_present_only_as_prose".to_string());
    }
    out
}

fn standards_row_exists(root: &Path) -> bool {
    json_boundary::read_json(&root.join("templates/agent-standards/enforcement.json"))
        .ok()
        .and_then(|value| value.get("rows").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(STANDARD_ID))
}

fn trace_entry_exists(root: &Path) -> bool {
    json_boundary::read_json(&root.join("docs/foundational-law-traceability.json"))
        .ok()
        .and_then(|value| value.get("entries").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("obligation_id").and_then(Value::as_str) == Some(TRACE_OBLIGATION_ID))
}
