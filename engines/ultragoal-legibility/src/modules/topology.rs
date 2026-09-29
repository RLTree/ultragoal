use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// Ported from UltraGoal namespace/source/topology/factoring.rs.
pub(super) fn check(paths: &BTreeSet<String>, roots: &BTreeSet<String>) -> Vec<String> {
    let mut failures = Vec::new();
    let mut prefixes: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    for path in paths {
        let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
        let stem = name.trim_end_matches(".rs");
        let integration = roots.contains(path)
            && Path::new(dir)
                .file_name()
                .is_some_and(|name| name == "tests");
        if integration {
            continue;
        }
        if stem != "mod" {
            let child_prefix = format!("{}/", path.trim_end_matches(".rs"));
            if paths.iter().any(|child| child.starts_with(&child_prefix)) {
                failures.push(format!(
                    "module_partial_factoring:{path}:move_root_to_mod_rs"
                ));
            }
        }
        if matches!(stem, "mod" | "lib" | "main") {
            continue;
        }
        if let Some((prefix, _)) = stem.split_once('_')
            && prefix.len() >= 2
        {
            prefixes.entry((dir, prefix)).or_default().push(path);
        }
    }
    for ((dir, prefix), examples) in prefixes {
        let namespace = if dir.is_empty() {
            prefix.to_string()
        } else {
            format!("{dir}/{prefix}")
        };
        let already_routed = dir.rsplit('/').next() == Some(prefix)
            || paths.contains(&format!("{namespace}.rs"))
            || paths
                .iter()
                .any(|path| path.starts_with(&format!("{namespace}/")));
        if examples.len() > 2 && !already_routed {
            failures.push(format!(
                "module_residual_prefix:{dir}:{prefix}:{}:promote_directory_module",
                examples.join(",")
            ));
        }
    }
    failures
}
