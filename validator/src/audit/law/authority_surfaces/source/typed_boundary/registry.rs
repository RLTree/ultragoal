use crate::audit::source_governance::rust_syntax::{AuthorityKind, RustSyntaxReport};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BoundaryRow {
    pub(crate) path: &'static str,
    pub(crate) symbol: &'static str,
    pub(crate) authorities: &'static [AuthorityKind],
}

const PROCESS: &[AuthorityKind] = &[AuthorityKind::Process];

pub(super) const BOUNDARY_ROWS: &[BoundaryRow] = &[BoundaryRow {
    path: "validator/src/cli/successor_public/strict/python_source_law_adapter/process.rs",
    symbol: "run",
    authorities: PROCESS,
}];

pub(super) fn validation_failures(
    rows: &[BoundaryRow],
    reports: &BTreeMap<String, RustSyntaxReport>,
) -> Vec<String> {
    let mut failures = Vec::new();
    let mut exact = BTreeSet::new();
    let mut symbols = BTreeMap::<(&str, &str), BTreeSet<Vec<AuthorityKind>>>::new();
    for row in rows {
        let mut kinds = row.authorities.to_vec();
        kinds.sort();
        kinds.dedup();
        if row.path.is_empty() || row.symbol.is_empty() || kinds.is_empty() {
            failures.push(format!(
                "typed_boundary_registry_row_incomplete:{}:{}",
                row.path, row.symbol
            ));
            continue;
        }
        if kinds.len() != row.authorities.len() {
            failures.push(format!(
                "typed_boundary_registry_authority_duplicate:{}:{}",
                row.path, row.symbol
            ));
        }
        let key = (row.path, row.symbol, kinds.clone());
        if !exact.insert(key) {
            failures.push(format!(
                "typed_boundary_registry_duplicate:{}:{}",
                row.path, row.symbol
            ));
        }
        symbols
            .entry((row.path, row.symbol))
            .or_default()
            .insert(kinds);
        let Some(report) = reports.get(row.path) else {
            failures.push(format!(
                "typed_boundary_registry_unknown_path:{}:{}",
                row.path, row.symbol
            ));
            continue;
        };
        let functions = super::function(report, row.symbol);
        if functions.len() != 1 {
            failures.push(format!(
                "typed_boundary_registry_symbol_ambiguous:{}:{}:{}",
                row.path,
                row.symbol,
                functions.len()
            ));
        } else if !functions[0].returns_closed_result {
            failures.push(format!(
                "typed_boundary_registry_open_result:{}:{}",
                row.path, row.symbol
            ));
        }
    }
    for ((path, symbol), variants) in symbols {
        if variants.len() > 1 {
            failures.push(format!("typed_boundary_registry_conflict:{path}:{symbol}"));
        }
    }
    failures
}

pub(super) fn permits(rows: &[BoundaryRow], path: &str, symbol: &str, kind: AuthorityKind) -> bool {
    rows.iter()
        .any(|row| row.path == path && row.symbol == symbol && row.authorities.contains(&kind))
}
