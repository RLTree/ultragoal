use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::path::Path;

use super::failure_text::remediating_failure;

#[cfg(test)]
pub(crate) fn failures(root: &Path, manifest_paths: &[String]) -> Vec<String> {
    failures_with_repo_paths(manifest_paths, &repo_source_paths(root))
}

pub(crate) fn failures_with_repo_paths(
    manifest_paths: &[String],
    repo_source_paths: &[String],
) -> Vec<String> {
    let mut paths = manifest_paths
        .iter()
        .filter(|path| is_validator_rust_source(path))
        .cloned()
        .collect::<BTreeSet<_>>();
    paths.extend(repo_source_paths.iter().cloned());
    let mut out = Vec::new();
    out.extend(forbidden_top_level_clusters(&paths));
    out.extend(partial_module_factoring_failures(&paths));
    out.extend(maximal_factoring_failures(&paths));
    out.extend(generic_leaf_name_failures(&paths));
    out.extend(history_name_failures(&paths));
    out.extend(opaque_gate_number_failures(&paths));
    out.extend(product_opaque_goal_work_label_failures(&paths));
    out
}

pub(crate) fn repo_source_paths_from_actual_files(actual_files: &[String]) -> Vec<String> {
    actual_files
        .iter()
        .filter(|path| is_validator_rust_source(path))
        .cloned()
        .collect()
}

#[cfg(test)]
fn repo_source_paths(root: &Path) -> Vec<String> {
    repo_source_paths_from_actual_files(
        &crate::package::inventory::closure::actual_files(root).unwrap_or_default(),
    )
}

fn forbidden_top_level_clusters(paths: &BTreeSet<String>) -> Vec<String> {
    let top_level = paths
        .iter()
        .filter_map(|path| path.strip_prefix("validator/src/"))
        .filter(|tail| !tail.contains('/'))
        .collect::<Vec<_>>();
    let internal = top_level
        .iter()
        .filter(|file| {
            file.starts_with("internal_")
                || file.starts_with("internal-")
                || file.starts_with("internal.")
                || file.starts_with("iinternal_")
        })
        .map(|file| format!("validator/src/{file}"))
        .collect::<Vec<_>>();
    if internal.is_empty() {
        return Vec::new();
    }
    vec![remediating_failure(
        "namespace_validator_source_top_level_internal_cluster",
        "validator/src",
        "internal",
        &internal,
        "move_repo_owned_validator_tests_into_validator_src_self_tests_semantic_domain_dirs",
        false,
    )]
}

fn partial_module_factoring_failures(paths: &BTreeSet<String>) -> Vec<String> {
    paths
        .iter()
        .filter(|path| source_stem(path) != "mod")
        .filter_map(|path| {
            let module_dir = path.strip_suffix(".rs")?;
            let child_prefix = format!("{module_dir}/");
            let has_child_source = paths.iter().any(|other| other.starts_with(&child_prefix));
            has_child_source.then(|| {
                remediating_failure(
                    "namespace_validator_source_partial_module_factoring",
                    parent_dir(path),
                    source_stem(path),
                    &[path.to_string()],
                    "move_partially_factored_module_root_to_mod_rs_so_the_directory_is_the_module_boundary",
                    false,
                )
            })
        })
        .collect()
}

fn maximal_factoring_failures(paths: &BTreeSet<String>) -> Vec<String> {
    let mut by_dir_prefix: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for path in paths {
        let stem = source_stem(path);
        if route_file(stem) || !stem.contains('_') {
            continue;
        }
        let prefix = stem.split('_').next().unwrap_or("");
        if prefix.len() < 2 {
            continue;
        }
        by_dir_prefix
            .entry((parent_dir(path).to_string(), prefix.to_string()))
            .or_default()
            .push(path.to_string());
    }
    by_dir_prefix
        .into_iter()
        .filter(|(_, examples)| examples.len() > 1)
        .map(|((dir, prefix), examples)| {
            remediating_failure(
                "namespace_validator_source_residual_prefix_encoding",
                &dir,
                &prefix,
                &examples,
                "promote_shared_underscore_namespace_segment_into_directory_module_boundary_until_filename_is_a_semantic_leaf",
                false,
            )
        })
        .collect()
}

fn generic_leaf_name_failures(paths: &BTreeSet<String>) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| {
            let label = super::path_labels::generic_source_leaf_label(source_stem(path))?;
            Some(remediating_failure(
                "namespace_validator_source_generic_leaf",
                parent_dir(path),
                label,
                &[path.to_string()],
                "rename_generic_source_leaf_to_the_domain_behavior_it_owns_or_route_it_under_a_semantic_module",
                false,
            ))
        })
        .collect()
}

fn history_name_failures(paths: &BTreeSet<String>) -> Vec<String> {
    paths
        .iter()
        .filter(|path| {
            let stem = source_stem(path);
            stem.starts_with("coverage_wave")
                || stem.contains("_coverage_wave")
                || stem.contains("_wave")
                || stem.starts_with("wave_")
                || stem.starts_with("iinternal_")
        })
        .map(|path| {
            remediating_failure(
                "namespace_validator_source_history_name",
                parent_dir(path),
                first_token(path),
                &[path.to_string()],
                "rename_repo_owned_validator_source_by_domain_behavior_not_creation_history",
                false,
            )
        })
        .collect()
}

fn opaque_gate_number_failures(paths: &BTreeSet<String>) -> Vec<String> {
    paths
        .iter()
        .filter(|path| {
            path.strip_prefix("validator/")
                .unwrap_or(path)
                .split(['/', '_', '-', '.'])
                .any(gate_number_token)
        })
        .map(|path| {
            remediating_failure(
                "namespace_validator_source_opaque_gate_number_name",
                parent_dir(path),
                first_token(path),
                &[path.to_string()],
                "rename_gate_number_source_to_the_domain_behavior_it_enforces",
                false,
            )
        })
        .collect()
}

fn product_opaque_goal_work_label_failures(paths: &BTreeSet<String>) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| {
            let label = super::path_labels::product_opaque_goal_work_label(path)?;
            Some(remediating_failure(
                "namespace_validator_source_product_opaque_goal_work_label",
                parent_dir(path),
                label,
                &[path.to_string()],
                "rename_source_path_by_cli_product_behavior_such_as_command_roundtrip_command_inventory_or_telemetry_reconciliation",
                false,
            ))
        })
        .collect()
}

fn gate_number_token(token: &str) -> bool {
    token
        .strip_prefix("gate")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_validator_rust_source(path: &str) -> bool {
    (path.starts_with("validator/src/") || path.starts_with("validator/tests/"))
        && path.ends_with(".rs")
}

fn route_file(stem: &str) -> bool {
    matches!(stem, "mod" | "lib" | "main")
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
}

fn first_token(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.split(['_', '-', '.']).next().unwrap_or("")
}

fn source_stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".rs").unwrap_or(name)
}
