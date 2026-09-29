//! Conservative source AST inventory, not compiler or macro-expansion proof.
mod alias_parameters;
mod classify;
mod closed_types;
#[cfg(test)]
mod closure_tests;
mod declarations;
mod functions;
mod generic_bounds;
mod imports;
mod macros;
mod model;
mod type_aliases;
mod visitor;
mod walk;

pub(crate) use model::*;
use syn::visit::Visit;

pub(crate) fn production_item(item: &syn::Item) -> bool {
    !declarations::test_only(item)
}

#[cfg(test)]
pub(crate) fn analyze(path: &str, source: &str) -> Result<Report, String> {
    analyze_with_object_contract(path, source, false)
}

pub(crate) fn object_contract_from_source(source: &str) -> bool {
    syn::parse_file(source)
        .ok()
        .and_then(|file| declarations::object_contract(&file.items))
        .unwrap_or(false)
}

#[cfg(test)]
pub(crate) fn analyze_with_object_contract(
    path: &str,
    source: &str,
    trusted: bool,
) -> Result<Report, String> {
    analyze_with_context(path, source, trusted, &[])
}

pub(crate) fn module_anyhow_aliases(
    source: &str,
    inherited: &[String],
) -> Result<Vec<String>, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let (local, _) = imports::collect(&file.items);
    let scopes = [type_aliases::inherited(inherited), local];
    let mut names = scopes
        .iter()
        .flat_map(|s| s.keys().cloned())
        .filter(|n| !n.contains("::") && n != "*")
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names.retain(|name| {
        syn::parse_str::<syn::Path>(name)
            .is_ok_and(|path| imports::resolve(&path, &scopes) == ["anyhow::Result"])
    });
    Ok(names)
}

#[cfg(test)]
pub(crate) fn analyze_with_context(
    path: &str,
    source: &str,
    trusted: bool,
    inherited_anyhow_aliases: &[String],
) -> Result<Report, String> {
    analyze_with_full_context(path, source, trusted, inherited_anyhow_aliases, &[])
}

#[cfg(test)]
pub(crate) fn analyze_with_full_context(
    path: &str,
    source: &str,
    trusted: bool,
    inherited_anyhow_aliases: &[String],
    inherited_type_aliases: &[(String, String)],
) -> Result<Report, String> {
    analyze_with_bindings(
        path,
        source,
        trusted,
        inherited_anyhow_aliases,
        inherited_type_aliases,
        &std::collections::BTreeMap::new(),
    )
}

pub(crate) fn analyze_with_bindings(
    path: &str,
    source: &str,
    trusted: bool,
    inherited_anyhow_aliases: &[String],
    inherited_type_aliases: &[(String, String)],
    bindings: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<Report, String> {
    let file = syn::parse_file(source).map_err(|error| format!("{path}: {error}"))?;
    let mut visitor = visitor::Scanner::default();
    visitor
        .scopes
        .push(crate::source_context::bindings::prelude());
    visitor.scopes.push(bindings.clone());
    visitor.type_aliases.extend([
        std::collections::BTreeMap::new(),
        std::collections::BTreeMap::new(),
    ]);
    let (scope, definitions) =
        type_aliases::context(inherited_anyhow_aliases, inherited_type_aliases)?;
    visitor.scopes.push(scope);
    visitor.type_aliases.push(definitions);
    visitor.object_contract.push(Some(trusted));
    visitor.report.limitations.push(
        "Production cfg(test) items excluded; other cfg branches retained; lexical imports resolved without compiler type inference or arbitrary macro expansion".into(),
    );
    visitor.report.limitations.push(
        "serde_saphyr parser detection is a project adaptation of structured_input authority"
            .into(),
    );
    visitor.scope(&file.items);
    visitor.report.limitations.push("Verified anyhow::Result<T> aliases use the known anyhow::Error default as a project adaptation".into());
    visitor.report.limitations.push("Unit and recursively named Option/Vec result responses are an approved domain-closure adaptation; plain named returns require separate explicit policy".into());
    visitor.visit_file(&file);
    Ok(visitor.finish())
}

pub(crate) fn module_type_aliases(
    source: &str,
    anyhow_aliases: &[String],
    inherited: &[(String, String)],
) -> Result<Vec<(String, String)>, String> {
    type_aliases::module_aliases(source, anyhow_aliases, inherited)
}

#[cfg(test)]
mod alias_tests;
#[cfg(test)]
mod composition_tests;
#[cfg(test)]
mod edge_tests;
#[cfg(test)]
mod tests;
