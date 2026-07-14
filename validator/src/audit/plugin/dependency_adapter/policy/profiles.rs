use super::super::model::{
    BoundaryKind, DependencyProfile, DependencyRow, DirectDependency, ProfileContract,
};
use super::reports;
use crate::audit::source_governance::rust_syntax::{AuthorityKind, RustSyntaxReport};
use std::collections::BTreeMap;

pub(super) fn failures(
    dependency: &DirectDependency,
    row: &DependencyRow,
    syntax: &BTreeMap<String, RustSyntaxReport>,
) -> Vec<String> {
    let mut failures = row
        .profiles
        .iter()
        .flat_map(|profile| profile_failures(dependency, profile, syntax))
        .collect::<Vec<_>>();
    let sites = reports::observed_sites(dependency, syntax);
    if sites.is_empty() {
        failures.push(format!(
            "dependency_adapter_direct_dependency_unobserved:{}",
            dependency.crate_name
        ));
    }
    for path in sites {
        let matches = row
            .profiles
            .iter()
            .filter(|profile| profile.contract.modules().contains(&path.as_str()))
            .collect::<Vec<_>>();
        match matches.len() {
            1 => {}
            0 => failures.push(format!(
                "dependency_adapter_direct_bypass:{}:{path}",
                dependency.crate_name
            )),
            count => failures.push(format!(
                "dependency_adapter_call_site_profile_ambiguous:{}:{path}:{count}",
                dependency.crate_name
            )),
        }
    }
    failures
}

fn profile_failures(
    dependency: &DirectDependency,
    profile: &DependencyProfile,
    syntax: &BTreeMap<String, RustSyntaxReport>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if profile.timeout.detail().trim().len() < 16 || profile.retry.detail().trim().len() < 16 {
        failures.push(format!(
            "dependency_adapter_profile_operational_policy_missing:{}:{}",
            dependency.crate_name, profile.adapter_id
        ));
    }
    if invalid_cache_inputs(&profile.cache_invalidation_inputs) {
        failures.push(format!(
            "dependency_adapter_profile_cache_inputs_invalid:{}:{}",
            dependency.crate_name, profile.adapter_id
        ));
    }
    let contract_matches = profile.boundary_kind.requires_owned_adapter()
        == matches!(profile.contract, ProfileContract::OwnedAdapter { .. });
    if !contract_matches {
        failures.push(format!(
            "dependency_adapter_profile_contract_kind_mismatch:{}:{}",
            dependency.crate_name, profile.adapter_id
        ));
        return failures;
    }
    match &profile.contract {
        ProfileContract::OwnedAdapter {
            module,
            request,
            response,
            error,
        } => {
            failures.extend(owned_failures(
                dependency,
                profile,
                module,
                [request.as_str(), response.as_str(), error.as_str()],
                syntax,
            ));
        }
        ProfileContract::TypedDeclarations { declarations } => {
            failures.extend(declaration_failures(
                dependency,
                profile,
                declarations,
                syntax,
            ));
        }
    }
    failures
}

fn owned_failures(
    dependency: &DirectDependency,
    profile: &DependencyProfile,
    module: &str,
    types: [&str; 3],
    syntax: &BTreeMap<String, RustSyntaxReport>,
) -> Vec<String> {
    if !exact_module(module) || types.iter().any(|name| name.is_empty()) {
        return vec![format!(
            "dependency_adapter_owned_contract_invalid:{}:{}",
            dependency.crate_name, profile.adapter_id
        )];
    }
    let Some(report) = syntax.get(module) else {
        return vec![format!(
            "dependency_adapter_module_missing:{}:{}:{module}",
            dependency.crate_name, profile.adapter_id
        )];
    };
    let mut failures = Vec::new();
    for (role, name) in ["request", "response", "error"].into_iter().zip(types) {
        if !report.closed_records.contains(name) {
            failures.push(format!(
                "dependency_adapter_closed_type_missing:{}:{}:{role}:{name}",
                dependency.crate_name, profile.adapter_id
            ));
        }
    }
    if !report
        .functions
        .iter()
        .any(|function| function.returns_closed_result)
    {
        failures.push(format!(
            "dependency_adapter_closed_operation_missing:{}:{}:{module}",
            dependency.crate_name, profile.adapter_id
        ));
    }
    for authority in &report.authorities {
        if raw_boundary_authority(authority.kind) {
            failures.push(format!(
                "dependency_adapter_raw_boundary_authority:{}:{}:{module}:{}",
                dependency.crate_name,
                profile.adapter_id,
                authority.kind.id()
            ));
        }
    }
    failures
}

fn declaration_failures(
    dependency: &DirectDependency,
    profile: &DependencyProfile,
    declarations: &[super::super::model::DeclarationSite],
    syntax: &BTreeMap<String, RustSyntaxReport>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if declarations.is_empty() {
        return vec![format!(
            "dependency_adapter_declarations_missing:{}:{}",
            dependency.crate_name, profile.adapter_id
        )];
    }
    for declaration in declarations {
        let Some(report) = syntax.get(&declaration.module) else {
            failures.push(format!(
                "dependency_adapter_declaration_module_missing:{}:{}:{}",
                dependency.crate_name, profile.adapter_id, declaration.module
            ));
            continue;
        };
        if !exact_module(&declaration.module) || !sorted_unique(&declaration.symbols) {
            failures.push(format!(
                "dependency_adapter_declaration_shape_invalid:{}:{}:{}",
                dependency.crate_name, profile.adapter_id, declaration.module
            ));
        }
        for symbol in &declaration.symbols {
            let exists = match profile.boundary_kind {
                BoundaryKind::TypedContractDerive => report.closed_records.contains(symbol),
                BoundaryKind::CliContractDeclaration => {
                    report.identifiers.iter().any(|row| row.name == *symbol)
                }
                _ => false,
            };
            if !exists {
                failures.push(format!(
                    "dependency_adapter_declaration_symbol_missing:{}:{}:{}:{symbol}",
                    dependency.crate_name, profile.adapter_id, declaration.module
                ));
            }
        }
    }
    failures
}

fn exact_module(path: &str) -> bool {
    path.starts_with("validator/src/")
        && path.ends_with(".rs")
        && !path.contains("//")
        && !path.contains("../")
}

fn raw_boundary_authority(kind: AuthorityKind) -> bool {
    matches!(
        kind,
        AuthorityKind::JsonMap
            | AuthorityKind::JsonValue
            | AuthorityKind::RawString
            | AuthorityKind::StringValueMap
            | AuthorityKind::TomlValue
    )
}

fn invalid_cache_inputs(values: &[String]) -> bool {
    values.len() < 2
        || !sorted_unique(values)
        || values.iter().any(|value| {
            value.is_empty()
                || value.bytes().any(|byte| {
                    !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
                })
        })
        || !values.iter().any(|value| value.contains("version"))
        || !values.iter().any(|value| {
            [
                "source",
                "input",
                "request",
                "artifact",
                "bytes",
                "call_site",
            ]
            .iter()
            .any(|token| value.contains(token))
        })
}

fn sorted_unique(values: &[String]) -> bool {
    !values.is_empty() && values.windows(2).all(|pair| pair[0] < pair[1])
}
