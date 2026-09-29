//! Direct AST calls only: this does not establish type checking or dynamic dispatch.
use super::resolution::{joined, local_namespace, location, resolve};
use super::standard::known_pure;
use crate::syntax;
use std::collections::{BTreeMap, BTreeSet};

type Site = (String, String);
pub(super) type Index = BTreeMap<(String, String), Vec<Site>>;

#[derive(Debug, Default)]
pub(super) struct Trace {
    pub(super) sites: Vec<Site>,
    pub(super) unknown: Vec<Unknown>,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct Unknown {
    pub(super) path: String,
    pub(super) symbol: String,
    pub(super) call: String,
    pub(super) reason: String,
}

fn unknown(path: &str, symbol: &str, call: &str, reason: &str) -> Unknown {
    Unknown {
        path: path.into(),
        symbol: symbol.into(),
        call: call.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
pub(super) fn trace(
    start_path: &str,
    start_symbol: &str,
    reports: &BTreeMap<String, syntax::Report>,
) -> Trace {
    trace_with_aliases(start_path, start_symbol, reports, &[])
}

#[cfg(test)]
pub(super) fn trace_with_aliases(
    start_path: &str,
    start_symbol: &str,
    reports: &BTreeMap<String, syntax::Report>,
    aliases: &[((String, String), Vec<Site>)],
) -> Trace {
    trace_with_boundaries(start_path, start_symbol, reports, aliases, &BTreeSet::new())
}

pub(super) fn trace_with_boundaries(
    start_path: &str,
    start_symbol: &str,
    reports: &BTreeMap<String, syntax::Report>,
    aliases: &[((String, String), Vec<Site>)],
    terminals: &BTreeSet<Site>,
) -> Trace {
    let mut index = Index::new();
    let mut constructors = BTreeSet::new();
    for (path, report) in reports {
        let Some((root, module)) = location(path) else {
            continue;
        };
        for function in &report.functions {
            index
                .entry((root.clone(), joined(&module, &function.name)))
                .or_default()
                .push((path.clone(), function.name.clone()));
            if let (Some(owner), Some((declared, method))) =
                (&function.self_type, function.name.rsplit_once("::"))
                && syn::parse_str::<syn::TypePath>(declared)
                    .ok()
                    .and_then(|p| p.path.segments.last().map(|s| s.ident.to_string()))
                    .as_deref()
                    == owner.rsplit("::").next()
            {
                let canonical = owner
                    .strip_prefix("crate::")
                    .map(str::to_string)
                    .unwrap_or_else(|| joined(&module, owner));
                index
                    .entry((root.clone(), format!("{canonical}::{method}")))
                    .or_default()
                    .push((path.clone(), function.name.clone()));
            }
        }
        for declaration in &report.types {
            constructors.insert((root.clone(), joined(&module, &declaration.name)));
            if declaration.derived_default {
                constructors.insert((
                    root.clone(),
                    joined(&module, &format!("{}::default", declaration.name)),
                ));
            }
            if let Some(target) = &declaration.aliased_path {
                for method in ["new", "default", "with_capacity"] {
                    if known_pure(&format!("{target}::{method}")) {
                        constructors.insert((
                            root.clone(),
                            joined(&module, &format!("{}::{method}", declaration.name)),
                        ));
                    }
                }
            }
            for variant in &declaration.variants {
                constructors.insert((
                    root.clone(),
                    joined(&module, &format!("{}::{variant}", declaration.name)),
                ));
            }
        }
    }
    let mut result = Trace::default();
    for (alias, sites) in aliases {
        let entries = index.entry(alias.clone()).or_default();
        entries.extend(sites.clone());
        entries.sort();
        entries.dedup();
    }
    let mut pending = vec![(start_path.to_string(), start_symbol.to_string())];
    let mut visited = BTreeSet::new();
    while let Some((path, symbol)) = pending.pop() {
        if !visited.insert((path.clone(), symbol.clone())) {
            continue;
        }
        let Some(report) = reports.get(&path) else {
            result
                .unknown
                .push(unknown(&path, &symbol, "", "source_missing"));
            continue;
        };
        let functions: Vec<_> = report
            .functions
            .iter()
            .filter(|f| f.name == symbol)
            .collect();
        if functions.is_empty()
            || (functions.len() != 1 && !crate::conditional::exclusive(&functions))
        {
            result
                .unknown
                .push(unknown(&path, &symbol, "", "producer_missing_or_ambiguous"));
            continue;
        }
        result.sites.push((path.clone(), symbol.clone()));
        if terminals.contains(&(path.clone(), symbol.clone())) {
            continue;
        }
        let Some((root, file_module)) = location(&path) else {
            result
                .unknown
                .push(unknown(&path, &symbol, "", "crate_root_unresolved"));
            continue;
        };
        let calls: BTreeSet<_> = functions.iter().flat_map(|f| &f.direct_calls).collect();
        for call in calls {
            let candidates = resolve(call, &symbol, &file_module, report);
            let constructor = candidates
                .iter()
                .any(|c| constructors.contains(&(root.clone(), c.clone())));
            let mut targets = Vec::new();
            for candidate in candidates {
                if let Some(found) = index.get(&(root.clone(), candidate)) {
                    targets.extend(found.clone());
                    // Bare calls use nearest lexical scope, then enclosing scopes.
                    if !call.contains("::") {
                        break;
                    }
                }
            }
            targets.sort();
            targets.dedup();
            match targets.len() {
                1 => pending.extend(targets),
                0 if constructor => {}
                0 if super::standard::local_observation_or_timing(call)
                    && !local_namespace(call, &root, &file_module, report, &index) => {}
                0 if known_pure(call)
                    && !local_namespace(call, &root, &file_module, report, &index) => {}
                0 => result
                    .unknown
                    .push(unknown(&path, &symbol, call, "unresolved_call")),
                _ => result
                    .unknown
                    .push(unknown(&path, &symbol, call, "ambiguous_call")),
            }
        }
    }
    result.sites.sort();
    result.unknown.sort();
    result.unknown.dedup();
    result
}
