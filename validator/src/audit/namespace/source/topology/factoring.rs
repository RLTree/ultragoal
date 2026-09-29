use std::collections::{BTreeMap, BTreeSet};

use super::super::failure_text::remediating_failure;

pub(super) fn partial_module_failures(paths: &BTreeSet<String>) -> Vec<String> {
    paths
        .iter()
        .filter(|path| source_stem(path) != "mod")
        .filter(|path| !cargo_integration_entrypoint(path))
        .filter_map(|path| {
            let module_dir = path.strip_suffix(".rs")?;
            let child_prefix = format!("{module_dir}/");
            paths
                .iter()
                .any(|other| other.starts_with(&child_prefix))
                .then(|| {
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

pub(super) fn maximal_prefix_failures(paths: &BTreeSet<String>) -> Vec<String> {
    let mut by_dir_prefix: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for path in paths {
        let stem = source_stem(path);
        if cargo_integration_entrypoint(path) || route_file(stem) || !stem.contains('_') {
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
        .filter(|((dir, prefix), examples)| {
            examples.len() > 2 && prefix_is_missing_namespace(paths, dir, prefix)
        })
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

fn cargo_integration_entrypoint(path: &str) -> bool {
    let Some(tail) = path.strip_prefix("validator/tests/") else {
        return false;
    };
    tail.ends_with(".rs") && !tail.contains('/')
}

fn prefix_is_missing_namespace(paths: &BTreeSet<String>, dir: &str, prefix: &str) -> bool {
    if dir.rsplit('/').next() == Some(prefix) {
        return false;
    }
    let module_root = format!("{dir}/{prefix}.rs");
    let module_dir = format!("{dir}/{prefix}/");
    !paths.contains(&module_root) && !paths.iter().any(|path| path.starts_with(&module_dir))
}

fn route_file(stem: &str) -> bool {
    matches!(stem, "mod" | "lib" | "main")
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
}

fn source_stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".rs").unwrap_or(name)
}
