mod module_path;

use super::cfg;
use super::model::CompilePossibility;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) struct OwnershipReport {
    pub(super) test_only: BTreeSet<String>,
    pub(super) failures: Vec<String>,
}

#[derive(Clone)]
struct Edge {
    target: String,
    possibility: CompilePossibility,
}

pub(super) fn analyze(files: &BTreeMap<String, syn::File>) -> OwnershipReport {
    let mut failures = Vec::new();
    let mut edges = BTreeMap::<String, Vec<Edge>>::new();
    let mut file_possibilities = BTreeMap::new();
    for (relative, file) in files {
        let file_possibility = cfg::possibility(&file.attrs).unwrap_or_else(|detail| {
            failures.push(format!("production_source_cfg_invalid:{relative}:{detail}"));
            CompilePossibility::BOTH
        });
        file_possibilities.insert(relative.clone(), file_possibility);
    }
    {
        let mut collector = ModuleCollector {
            files,
            edges: &mut edges,
            failures: &mut failures,
        };
        for (relative, file) in files {
            let (path_base, module_base) = module_path::source_bases(relative);
            collector.collect(
                relative,
                &path_base,
                &module_base,
                &file.items,
                CompilePossibility::BOTH,
            );
        }
    }
    let cycle_nodes = cycle_nodes(&edges, &mut failures);
    let mut incoming = files
        .keys()
        .map(|relative| (relative.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    for edge in edges.values().flatten() {
        if let Some(count) = incoming.get_mut(&edge.target) {
            *count += 1;
        }
    }
    let mut reachable = files
        .keys()
        .map(|relative| {
            (
                relative.clone(),
                CompilePossibility {
                    production: false,
                    test: false,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (relative, count) in &incoming {
        if *count == 0 {
            reachable.insert(relative.clone(), file_possibilities[relative]);
        }
    }
    propagate(&edges, &file_possibilities, &mut reachable);
    for relative in files.keys() {
        let state = reachable[relative];
        if (!state.production && !state.test) || cycle_nodes.contains(relative) {
            reachable
                .get_mut(relative)
                .expect("known source")
                .production = true;
        }
    }
    propagate(&edges, &file_possibilities, &mut reachable);
    let test_only = reachable
        .into_iter()
        .filter_map(|(relative, state)| (state.test && !state.production).then_some(relative))
        .collect();
    failures.sort();
    failures.dedup();
    OwnershipReport {
        test_only,
        failures,
    }
}

struct ModuleCollector<'a> {
    files: &'a BTreeMap<String, syn::File>,
    edges: &'a mut BTreeMap<String, Vec<Edge>>,
    failures: &'a mut Vec<String>,
}

impl ModuleCollector<'_> {
    fn collect(
        &mut self,
        owner: &str,
        path_base: &Path,
        module_base: &Path,
        items: &[syn::Item],
        inherited: CompilePossibility,
    ) {
        for item in items {
            let syn::Item::Mod(module) = item else {
                continue;
            };
            let item_possibility = cfg::possibility(&module.attrs).unwrap_or_else(|detail| {
                self.failures
                    .push(format!("production_source_cfg_invalid:{owner}:{detail}"));
                CompilePossibility::BOTH
            });
            let possibility = inherited.and(item_possibility);
            if let Some((_, nested)) = &module.content {
                self.collect(
                    owner,
                    path_base,
                    &module_base.join(module.ident.to_string()),
                    nested,
                    possibility,
                );
                continue;
            }
            let path_override = match cfg::path_attribute(&module.attrs) {
                Ok(value) => value,
                Err(detail) => {
                    self.failures
                        .push(format!("production_source_path_invalid:{owner}:{detail}"));
                    continue;
                }
            };
            let candidates = if let Some(path) = path_override {
                module_path::normalize(path_base.join(path))
                    .into_iter()
                    .collect()
            } else {
                module_path::conventional_candidates(module_base, &module.ident.to_string())
            };
            let matches = candidates
                .into_iter()
                .filter(|candidate| self.files.contains_key(candidate))
                .collect::<Vec<_>>();
            if matches.len() > 1 {
                self.failures.push(format!(
                    "production_source_module_ambiguous:{owner}:{}",
                    module.ident
                ));
            } else if let Some(target) = matches.into_iter().next() {
                self.edges.entry(owner.to_string()).or_default().push(Edge {
                    target,
                    possibility,
                });
            }
        }
    }
}

fn propagate(
    edges: &BTreeMap<String, Vec<Edge>>,
    file_possibilities: &BTreeMap<String, CompilePossibility>,
    reachable: &mut BTreeMap<String, CompilePossibility>,
) {
    let mut changed = true;
    while changed {
        changed = false;
        for (owner, owner_edges) in edges {
            let owner_state = reachable[owner];
            for edge in owner_edges {
                let file = file_possibilities[&edge.target];
                let candidate = CompilePossibility {
                    production: owner_state.production
                        && edge.possibility.production
                        && file.production,
                    test: owner_state.test && edge.possibility.test && file.test,
                };
                let target = reachable.get_mut(&edge.target).expect("known module");
                let next = CompilePossibility {
                    production: target.production || candidate.production,
                    test: target.test || candidate.test,
                };
                if *target != next {
                    *target = next;
                    changed = true;
                }
            }
        }
    }
}

fn cycle_nodes(
    edges: &BTreeMap<String, Vec<Edge>>,
    failures: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut visiting = Vec::new();
    let mut visited = BTreeSet::new();
    let mut cycles = BTreeSet::new();
    for node in edges.keys() {
        visit_cycle(
            node,
            edges,
            &mut visiting,
            &mut visited,
            &mut cycles,
            failures,
        );
    }
    cycles
}

fn visit_cycle(
    node: &str,
    edges: &BTreeMap<String, Vec<Edge>>,
    visiting: &mut Vec<String>,
    visited: &mut BTreeSet<String>,
    cycles: &mut BTreeSet<String>,
    failures: &mut Vec<String>,
) {
    if let Some(start) = visiting.iter().position(|value| value == node) {
        for member in &visiting[start..] {
            cycles.insert(member.clone());
        }
        failures.push(format!("production_source_module_cycle:{node}"));
        return;
    }
    if !visited.insert(node.to_string()) {
        return;
    }
    visiting.push(node.to_string());
    for edge in edges.get(node).into_iter().flatten() {
        visit_cycle(&edge.target, edges, visiting, visited, cycles, failures);
    }
    visiting.pop();
}
