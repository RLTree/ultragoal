use crate::{dependencies::named_symbol, inventory::safe_path, model::*, syntax::Report};
use std::collections::{BTreeMap, BTreeSet};
pub fn check(
    row: &DependencyRow,
    profile: &DependencyProfile,
    reports: &BTreeMap<String, Report>,
    boundaries: &[BoundaryRow],
    test_only: &BTreeSet<String>,
) -> Vec<String> {
    let mut failures = vec![];
    let adapter = matches!(
        profile.boundary_kind.as_str(),
        "typed_parser"
            | "typed_emitter"
            | "effect_adapter"
            | "crypto_adapter"
            | "filesystem_adapter"
    );
    let declaration = matches!(
        profile.boundary_kind.as_str(),
        "typed_contract_derive" | "cli_contract_declaration"
    );
    match &profile.contract {
        ProfileContract::FunctionAdapters { module, functions } => {
            if !adapter || !safe_path(module) || functions.is_empty() {
                failures.push(format!(
                    "dependency_function_adapters_invalid:{}:{module}",
                    row.crate_name
                ));
            }
            let mut seen = BTreeSet::new();
            for symbol in functions {
                let matches: Vec<_> = reports
                    .get(module)
                    .into_iter()
                    .flat_map(|r| &r.functions)
                    .filter(|f| &f.name == symbol)
                    .collect();
                if !seen.insert(symbol)
                    || (matches.len() != 1 && !crate::conditional::exclusive(&matches))
                    || !(test_only.contains(module)
                        || boundaries.iter().any(|b| {
                            b.path == *module
                                && b.symbol == *symbol
                                && !matches.is_empty()
                                && matches
                                    .iter()
                                    .all(|f| crate::boundaries::closed(b, f, reports))
                        }))
                {
                    failures.push(format!(
                        "dependency_adapter_not_closed_boundary:{}:{module}:{symbol}",
                        row.crate_name
                    ));
                }
            }
        }
        ProfileContract::PureOperations { module, symbols } => {
            if profile.boundary_kind != "pure_operations"
                || !safe_path(module)
                || !reports.contains_key(module)
                || symbols.is_empty()
            {
                failures.push(format!(
                    "dependency_pure_profile_invalid:{}:{module}",
                    row.crate_name
                ));
            }
            for symbol in symbols {
                if !crate::pure::permitted(symbol)
                    || symbol.split("::").next() != Some(row.crate_name.as_str())
                {
                    failures.push(format!(
                        "dependency_impure_operation:{}:{module}:{symbol}",
                        row.crate_name
                    ));
                }
            }
        }
        ProfileContract::OwnedAdapter {
            module,
            request,
            response,
            error,
        } => {
            if !adapter || !safe_path(module) || !reports.contains_key(module) {
                failures.push(format!(
                    "dependency_adapter_unresolved:{}:{module}",
                    row.crate_name
                ));
            }
            for symbol in [request, response, error] {
                if test_only.contains(module) {
                    continue;
                }
                if !named_symbol(symbol, reports) {
                    failures.push(format!(
                        "dependency_adapter_type_unresolved:{}:{module}:{symbol}",
                        row.crate_name
                    ));
                }
            }
        }
        ProfileContract::TypedDeclarations { declarations } => {
            if !declaration || declarations.is_empty() {
                failures.push(format!(
                    "dependency_declarations_invalid:{}",
                    row.crate_name
                ));
            }
            for site in declarations {
                let report = reports.get(&site.module);
                if !safe_path(&site.module) || report.is_none() || site.symbols.is_empty() {
                    failures.push(format!(
                        "dependency_declaration_unresolved:{}:{}",
                        row.crate_name, site.module
                    ));
                }
                for symbol in &site.symbols {
                    if !report.is_some_and(|r| r.types.iter().any(|t| &t.name == symbol)) {
                        failures.push(format!(
                            "dependency_declaration_symbol_missing:{}:{}:{symbol}",
                            row.crate_name, site.module
                        ));
                    }
                }
            }
        }
    }
    failures
}
