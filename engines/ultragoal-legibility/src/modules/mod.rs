//! Static module ownership; every cfg branch is inspected without macro expansion.
mod attributes;
mod production;
#[cfg(test)]
mod production_tests;
mod resolution;
#[cfg(test)]
mod tests;
mod topology;

pub(crate) use production::production_paths;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Complete module ownership audit, including all path and topology refusals.
pub(crate) struct ModuleAudit {
    pub(crate) failures: Vec<String>,
}

pub(crate) fn check(root: &Path, rust_files: &[String], entrypoints: &[String]) -> ModuleAudit {
    let inventory: BTreeSet<_> = rust_files.iter().cloned().collect();
    let roots: BTreeSet<_> = entrypoints.iter().cloned().collect();
    let mut failures = topology::check(&inventory, &roots);
    let mut edges = BTreeMap::new();
    for source in &inventory {
        let safe = match resolution::safe_path(root, Path::new(source)) {
            Ok(safe) => safe,
            Err(reason) => {
                failures.push(format!("module_path_invalid:{source}:{reason}"));
                continue;
            }
        };
        let parsed = std::fs::read_to_string(root.join(safe.as_path()))
            .map_err(|error| error.to_string())
            .and_then(|text| syn::parse_file(&text).map_err(|error| error.to_string()));
        match parsed {
            Ok(file) => {
                edges.insert(
                    source.clone(),
                    resolution::collect(
                        root,
                        source,
                        &file,
                        &inventory,
                        roots.contains(source),
                        &mut failures,
                    ),
                );
            }
            Err(error) => failures.push(format!("module_parse_failed:{source}:{error}")),
        }
    }
    let mut reached = BTreeSet::new();
    for entry in &roots {
        if !inventory.contains(entry) {
            failures.push(format!("module_entrypoint_missing:{entry}"));
        } else {
            reachable(entry, &edges, &mut reached);
        }
    }
    for orphan in inventory.difference(&reached) {
        failures.push(format!("module_orphan:{orphan}"));
    }
    let mut visited = BTreeSet::new();
    for source in &inventory {
        cycles(source, &edges, &mut Vec::new(), &mut visited, &mut failures);
    }
    failures.sort();
    failures.dedup();
    ModuleAudit { failures }
}

fn reachable(node: &str, edges: &BTreeMap<String, Vec<String>>, seen: &mut BTreeSet<String>) {
    if seen.insert(node.to_string()) {
        for target in edges.get(node).into_iter().flatten() {
            reachable(target, edges, seen);
        }
    }
}

fn cycles(
    node: &str,
    edges: &BTreeMap<String, Vec<String>>,
    active: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    failures: &mut Vec<String>,
) {
    if let Some(start) = active.iter().position(|value| value == node) {
        failures.push(format!(
            "module_cycle:{} -> {node}",
            active[start..].join(" -> ")
        ));
        return;
    }
    if !seen.insert(node.to_string()) {
        return;
    }
    active.push(node.to_string());
    for target in edges.get(node).into_iter().flatten() {
        cycles(target, edges, active, seen, failures);
    }
    active.pop();
}
