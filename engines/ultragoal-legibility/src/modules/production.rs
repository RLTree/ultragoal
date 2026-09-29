//! Port of UltraGoal production_source cfg possibility and ownership propagation.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use syn::{Attribute, Item, Meta, Token, punctuated::Punctuated};

type Edges = BTreeMap<String, Vec<String>>;

/// Source ownership classification and every refusal encountered deriving it.
pub(crate) struct ProductionSourceSet {
    pub(crate) test_only: BTreeSet<String>,
    pub(crate) failures: Vec<String>,
}

pub(crate) fn production_paths(
    root: &Path,
    rust_files: &[String],
    entrypoints: &[String],
) -> ProductionSourceSet {
    let inventory: BTreeSet<_> = rust_files.iter().cloned().collect();
    let entries: BTreeSet<_> = entrypoints.iter().cloned().collect();
    let mut failures = Vec::new();
    let mut parsed = BTreeMap::new();
    for path in &inventory {
        let result = super::resolution::safe_path(root, Path::new(path))
            .map_err(|error| error.to_string())
            .and_then(|safe| {
                std::fs::read_to_string(root.join(safe.as_path())).map_err(|e| e.to_string())
            })
            .and_then(|source| syn::parse_file(&source).map_err(|e| e.to_string()));
        match result {
            Ok(file) => {
                parsed.insert(path.clone(), file);
            }
            Err(error) => failures.push(format!("production_source_parse_invalid:{path}:{error}")),
        }
    }
    let mut reached = [BTreeSet::new(), BTreeSet::new()];
    let mut graphs = [Edges::new(), Edges::new()];
    for mode in 0..2 {
        let test = mode == 1;
        let mut enabled = BTreeSet::new();
        for (path, file) in &parsed {
            if !possible(&file.attrs, test, path, &mut failures) {
                continue;
            }
            enabled.insert(path.clone());
            let mut filtered = file.clone();
            filtered.items = modules(&file.items, test, path, &mut failures);
            graphs[mode].insert(
                path.clone(),
                super::resolution::collect(
                    root,
                    path,
                    &filtered,
                    &inventory,
                    entries.contains(path),
                    &mut failures,
                ),
            );
        }
        for targets in graphs[mode].values_mut() {
            targets.retain(|path| enabled.contains(path));
        }
        for entry in &entries {
            let integration = entry.starts_with("tests/") || entry.contains("/tests/");
            if enabled.contains(entry) && (test || !integration) {
                super::reachable(entry, &graphs[mode], &mut reached[mode]);
            }
        }
    }
    // As in UltraGoal, unowned/inactive files and cycles cannot gain a test exemption.
    let all_edges: Edges = inventory
        .iter()
        .map(|path| {
            let targets = graphs
                .iter()
                .flat_map(|g| g.get(path).into_iter().flatten())
                .cloned()
                .collect();
            (path.clone(), targets)
        })
        .collect();
    let mut uncertain: BTreeSet<_> = inventory
        .iter()
        .filter(|path| !reached[0].contains(*path) && !reached[1].contains(*path))
        .cloned()
        .collect();
    let mut seen = BTreeSet::new();
    for path in &inventory {
        cycle_nodes(
            path,
            &all_edges,
            &mut Vec::new(),
            &mut seen,
            &mut uncertain,
            &mut failures,
        );
    }
    for path in uncertain {
        super::reachable(&path, &graphs[0], &mut reached[0]);
    }
    for entry in entries.difference(&inventory) {
        failures.push(format!("production_source_entrypoint_missing:{entry}"));
    }
    failures.sort();
    failures.dedup();
    ProductionSourceSet {
        test_only: reached[1].difference(&reached[0]).cloned().collect(),
        failures,
    }
}

fn modules(items: &[Item], test: bool, path: &str, failures: &mut Vec<String>) -> Vec<Item> {
    items
        .iter()
        .filter_map(|item| {
            let Item::Mod(module) = item else {
                return None;
            };
            if !possible(&module.attrs, test, path, failures) {
                return None;
            }
            let mut module = module.clone();
            if let Some((_, children)) = &mut module.content {
                *children = modules(children, test, path, failures);
            }
            Some(Item::Mod(module))
        })
        .collect()
}

fn possible(attrs: &[Attribute], test: bool, path: &str, failures: &mut Vec<String>) -> bool {
    let result = (|| {
        super::attributes::path(attrs)?;
        let mut can = true;
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("cfg")) {
            let values = attr
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .map_err(|_| "cfg_predicate_invalid")?;
            if values.len() != 1 {
                return Err("cfg_predicate_arity_invalid");
            }
            can &= truth(&values[0], test)?.0;
        }
        Ok(can)
    })();
    result.unwrap_or_else(|detail: &str| {
        failures.push(format!("production_source_cfg_invalid:{path}:{detail}"));
        true
    })
}

// (can be true, can be false); unknown platform/features stay conservative.
fn truth(meta: &Meta, test: bool) -> Result<(bool, bool), &'static str> {
    match meta {
        Meta::Path(path) if path.is_ident("test") => Ok((test, !test)),
        Meta::List(list)
            if ["all", "any", "not"]
                .iter()
                .any(|name| list.path.is_ident(name)) =>
        {
            let values = list
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .map_err(|_| "cfg_nested_predicate_invalid")?;
            let states = values
                .iter()
                .map(|value| truth(value, test))
                .collect::<Result<Vec<_>, _>>()?;
            if list.path.is_ident("all") {
                Ok((states.iter().all(|s| s.0), states.iter().any(|s| s.1)))
            } else if list.path.is_ident("any") {
                Ok((states.iter().any(|s| s.0), states.iter().all(|s| s.1)))
            } else if states.len() == 1 {
                Ok((states[0].1, states[0].0))
            } else {
                Err("cfg_not_arity_invalid")
            }
        }
        _ => Ok((true, true)),
    }
}

fn cycle_nodes(
    node: &str,
    edges: &Edges,
    active: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    uncertain: &mut BTreeSet<String>,
    failures: &mut Vec<String>,
) {
    if let Some(start) = active.iter().position(|path| path == node) {
        uncertain.extend(active[start..].iter().cloned());
        failures.push(format!("production_source_module_cycle:{node}"));
        return;
    }
    if !seen.insert(node.to_string()) {
        return;
    }
    active.push(node.to_string());
    for target in edges.get(node).into_iter().flatten() {
        cycle_nodes(target, edges, active, seen, uncertain, failures);
    }
    active.pop();
}
