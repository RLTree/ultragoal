use crate::{inventory::safe_path, model::BoundaryRow, syntax::Report};
use std::collections::{BTreeMap, BTreeSet};

pub fn check(rows: &[BoundaryRow], reports: &BTreeMap<String, Report>) -> Vec<String> {
    check_with_outputs(rows, reports, &BTreeSet::new())
}

pub fn check_with_outputs(
    rows: &[BoundaryRow],
    reports: &BTreeMap<String, Report>,
    outputs: &BTreeSet<(String, String)>,
) -> Vec<String> {
    let mut failures = vec![];
    let mut seen = BTreeSet::new();
    for row in rows {
        let key = (&row.path, &row.symbol);
        if !safe_path(&row.path)
            || row.symbol.is_empty()
            || row.symbol.contains('*')
            || !seen.insert(key)
        {
            failures.push(format!(
                "boundary_registry_identity_invalid:{}:{}",
                row.path, row.symbol
            ));
        }
        let Some(report) = reports.get(&row.path) else {
            failures.push(format!("boundary_registry_unknown_path:{}", row.path));
            continue;
        };
        let functions: Vec<_> = report
            .functions
            .iter()
            .filter(|f| f.name == row.symbol)
            .collect();
        if functions.is_empty()
            || (functions.len() != 1 && !crate::conditional::exclusive(&functions))
        {
            failures.push(format!(
                "boundary_registry_symbol_ambiguous:{}:{}:{}",
                row.path,
                row.symbol,
                functions.len()
            ));
            continue;
        }
        let function = functions[0];
        if !functions.iter().all(|f| closed(row, f, reports)) {
            failures.push(format!(
                "boundary_open_result:{}:{}:{}",
                row.path, row.symbol, function.return_type
            ));
        }
        let mut kinds = BTreeSet::new();
        if row.authorities.is_empty() {
            failures.push(format!(
                "boundary_authorities_empty:{}:{}",
                row.path, row.symbol
            ));
        }
        for kind in &row.authorities {
            if !kinds.insert(kind)
                || !matches!(
                    kind.as_str(),
                    "environment"
                        | "serde_json_map"
                        | "serde_json_value"
                        | "process"
                        | "filesystem"
                        | "path"
                        | "path_buf"
                        | "string"
                        | "structured_input"
                        | "string_value_map"
                        | "toml_value"
                )
            {
                failures.push(format!(
                    "boundary_authority_invalid:{}:{}:{kind}",
                    row.path, row.symbol
                ));
            }
            if !report
                .authorities
                .iter()
                .any(|a| a.function.as_deref() == Some(&row.symbol) && &a.kind == kind)
            {
                failures.push(format!(
                    "boundary_registry_stale_authority:{}:{}:{kind}",
                    row.path, row.symbol
                ));
            }
        }
        // These additional exact declarations describe the reviewed domain validation route.
        for symbol in [&row.response, &row.error] {
            if !symbol.is_empty() {
                let last = symbol.rsplit("::").next().unwrap();
                if !function
                    .output_identifiers
                    .iter()
                    .any(|s| s.rsplit("::").next() == Some(last))
                    && function.resolved_error.as_ref() != Some(symbol)
                {
                    failures.push(format!(
                        "boundary_declared_result_not_returned:{}:{}:{symbol}",
                        row.path, row.symbol
                    ));
                }
            }
        }
        for validation in &row.validation {
            if validation.is_empty() || !function.calls.contains(validation) {
                failures.push(format!(
                    "boundary_validation_call_missing:{}:{}:{validation}",
                    row.path, row.symbol
                ));
            }
        }
    }
    for (path, report) in reports {
        for authority in &report.authorities {
            if crate::outputs::permits(outputs, path, authority.owner.as_deref(), &authority.kind) {
                continue;
            }
            let Some(function) = authority.function.as_deref() else {
                failures.push(format!(
                    "raw_authority_outside_boundary:{path}:{}",
                    authority.kind
                ));
                continue;
            };
            if !rows.iter().any(|r| {
                &r.path == path && r.symbol == function && r.authorities.contains(&authority.kind)
            }) {
                failures.push(format!(
                    "raw_authority_unregistered:{path}:{function}:{}",
                    authority.kind
                ));
            }
        }
    }
    failures
}

pub fn closed(
    row: &BoundaryRow,
    function: &crate::syntax::Function,
    reports: &BTreeMap<String, Report>,
) -> bool {
    function.returns_closed_result || crate::domain_response::closed(row, function, reports)
}
