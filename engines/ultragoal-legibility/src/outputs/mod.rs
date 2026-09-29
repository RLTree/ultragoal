mod graph;
#[cfg(test)]
mod graph_tests;
mod references;
mod resolution;
#[cfg(test)]
mod resolver_tests;
mod standard;
#[cfg(test)]
mod tests;

use crate::{
    inventory::{Inventory, safe_path},
    model::*,
    syntax::Report,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Checked {
    pub allowed: BTreeSet<(String, String)>,
    pub failures: Vec<String>,
}

pub fn check(
    rows: &[OutputRow],
    boundary_rows: &[BoundaryRow],
    dependencies: &[DependencyRow],
    reports: &BTreeMap<String, Report>,
    inventory: &Inventory,
) -> Checked {
    let mut checked = Checked {
        allowed: BTreeSet::new(),
        failures: vec![],
    };
    let aliases = crate::source_context::aliases(&inventory.files, reports);
    let terminals: BTreeSet<_> = boundary_rows
        .iter()
        .filter_map(|row| {
            let report = reports.get(&row.path)?;
            let functions: Vec<_> = report
                .functions
                .iter()
                .filter(|f| f.name == row.symbol)
                .collect();
            let unique = functions.len() == 1 || crate::conditional::exclusive(&functions);
            let closed = unique
                && !functions.is_empty()
                && functions
                    .iter()
                    .all(|f| crate::boundaries::closed(row, f, reports));
            let owned = report
                .authorities
                .iter()
                .filter(|a| a.function.as_deref() == Some(&row.symbol))
                .all(|a| row.authorities.contains(&a.kind));
            (closed && owned).then(|| (row.path.clone(), row.symbol.clone()))
        })
        .collect();
    for row in rows {
        let prior = checked.failures.len();
        let site = (row.path.clone(), row.symbol.clone());
        if !safe_path(&row.path)
            || row.symbol.is_empty()
            || row.symbol.contains('*')
            || !safe_path(&row.owner)
            || !inventory.files.contains_key(&row.owner)
            || row.purpose.trim().is_empty()
        {
            checked.failures.push(format!(
                "output_registry_incomplete:{}:{}",
                row.path, row.symbol
            ));
        }
        let Some(report) = reports.get(&row.path) else {
            checked.failures.push(format!(
                "output_site_unresolved:{}:{}",
                row.path, row.symbol
            ));
            continue;
        };
        let declaration = report.types.iter().any(|t| t.name == row.symbol);
        if !declaration && (row.producer.path != row.path || row.producer.symbol != row.symbol) {
            checked.failures.push(format!(
                "output_function_producer_mismatch:{}:{}",
                row.path, row.symbol
            ));
        }
        if !declaration
            && report
                .functions
                .iter()
                .filter(|f| f.name == row.symbol)
                .count()
                != 1
        {
            checked.failures.push(format!(
                "output_site_unresolved:{}:{}",
                row.path, row.symbol
            ));
        }
        let producer = references::function(&row.producer, reports);
        if producer.is_none() {
            checked.failures.push(format!(
                "output_producer_unresolved:{}:{}",
                row.producer.path, row.producer.symbol
            ));
        }
        if declaration
            && !producer.is_some_and(|f| {
                references::produces(&row.path, &row.symbol, &row.producer.path, f, reports)
            })
        {
            checked.failures.push(format!(
                "output_producer_does_not_return_declaration:{}:{}",
                row.path, row.symbol
            ));
        }
        for validator in &row.validators {
            if references::function(validator, reports).is_none() {
                checked.failures.push(format!(
                    "output_validator_unresolved:{}:{}",
                    validator.path, validator.symbol
                ));
            }
        }
        if row.tests.is_empty() {
            checked.failures.push(format!(
                "output_behavior_test_missing:{}:{}",
                row.path, row.symbol
            ));
        }
        for test in &row.tests {
            if !references::is_test(test, inventory) {
                checked.failures.push(format!(
                    "output_behavior_test_unresolved:{}:{}",
                    test.path, test.symbol
                ));
            }
        }
        let trace = graph::trace_with_boundaries(
            &row.producer.path,
            &row.producer.symbol,
            reports,
            &aliases,
            &terminals,
        );
        for (path, symbol) in &trace.sites {
            let report = &reports[path];
            for authority in report
                .authorities
                .iter()
                .filter(|a| a.function.as_deref() == Some(symbol))
            {
                if effect(&authority.kind)
                    && !owned_boundary(path, symbol, &authority.kind, boundary_rows, reports)
                {
                    checked.failures.push(format!(
                        "output_producer_unowned_effect:{path}:{symbol}:{}",
                        authority.kind
                    ));
                }
            }
        }
        for unknown in trace.unknown {
            let direct_kind = if unknown.call.starts_with("std::env::") {
                Some("environment")
            } else if unknown.call.starts_with("std::process::") {
                Some("process")
            } else if unknown.call.starts_with("std::fs::") {
                Some("filesystem")
            } else {
                None
            };
            let owned =
                direct_kind.is_some_and(|kind| {
                    owned_boundary(&unknown.path, &unknown.symbol, kind, boundary_rows, reports)
                }) || dependency_owned(&unknown.path, &unknown.symbol, &unknown.call, dependencies);
            if !owned {
                checked.failures.push(format!(
                    "output_producer_call_unresolved:{}:{}:{}:{}",
                    unknown.path, unknown.symbol, unknown.call, unknown.reason
                ));
            }
        }
        if prior == checked.failures.len() && !checked.allowed.insert(site) {
            checked.failures.push(format!(
                "output_registry_duplicate:{}:{}",
                row.path, row.symbol
            ));
        }
    }
    checked
}

fn dependency_owned(path: &str, symbol: &str, call: &str, rows: &[DependencyRow]) -> bool {
    let Some((name, _)) = call.split_once("::") else {
        return false;
    };
    rows.iter()
        .filter(|r| r.crate_name == name)
        .flat_map(|r| &r.profiles)
        .any(|p| match &p.contract {
            ProfileContract::OwnedAdapter { module, .. } => module == path,
            ProfileContract::PureOperations { module, symbols } => {
                module == path && symbols.iter().any(|s| s == call) && crate::pure::permitted(call)
            }
            ProfileContract::FunctionAdapters { module, functions } => {
                module == path && functions.iter().any(|s| s == symbol)
            }
            ProfileContract::TypedDeclarations { .. } => false,
        })
}

pub fn effect(kind: &str) -> bool {
    matches!(
        kind,
        "structured_input" | "process" | "environment" | "filesystem"
    )
}

fn owned_boundary(
    path: &str,
    symbol: &str,
    kind: &str,
    rows: &[BoundaryRow],
    reports: &BTreeMap<String, Report>,
) -> bool {
    rows.iter().any(|r| {
        r.path == path
            && r.symbol == symbol
            && r.authorities.iter().any(|k| k == kind)
            && reports.get(path).is_some_and(|report| {
                report
                    .functions
                    .iter()
                    .any(|f| f.name == symbol && crate::boundaries::closed(r, f, reports))
            })
    })
}

pub fn permits(
    allowed: &BTreeSet<(String, String)>,
    path: &str,
    owner: Option<&str>,
    kind: &str,
) -> bool {
    !effect(kind) && owner.is_some_and(|owner| allowed.contains(&(path.into(), owner.into())))
}
