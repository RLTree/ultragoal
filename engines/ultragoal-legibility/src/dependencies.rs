use crate::{
    inventory::{Inventory, safe_path},
    metadata::Metadata,
    model::*,
    syntax::Report,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn check_with_scope(
    rows: &[DependencyRow],
    metadata: &Metadata,
    reports: &BTreeMap<String, Report>,
    inventory: &Inventory,
    boundaries: &[BoundaryRow],
    test_only: &BTreeSet<String>,
) -> Vec<String> {
    let mut failures = vec![];
    let mut seen = BTreeSet::new();
    for row in rows {
        let key = (&row.manifest, &row.crate_name);
        if !seen.insert(key) {
            failures.push(format!(
                "dependency_registry_duplicate:{}:{}",
                row.manifest, row.crate_name
            ));
        }
        if !metadata.dependencies.iter().any(|d| {
            d.manifest == row.manifest
                && d.crate_name == row.crate_name
                && d.requirement == row.version_requirement
        }) {
            failures.push(format!(
                "dependency_version_or_manifest_mismatch:{}:{}:{}",
                row.manifest, row.crate_name, row.version_requirement
            ));
        }
        if row.purpose.trim().is_empty()
            || !row.upstream_docs.starts_with("https://")
            || !safe_path(&row.owner)
            || !inventory.files.contains_key(&row.owner)
        {
            failures.push(format!(
                "dependency_ownership_incomplete:{}",
                row.crate_name
            ));
        }
        let used_in_production = reports.iter().any(|(p, r)| {
            !test_only.contains(p)
                && r.dependencies
                    .iter()
                    .any(|d| d.crate_name == row.crate_name)
        });
        if row.profiles.is_empty() && (!test_only.contains(&row.owner) || used_in_production) {
            failures.push(format!("dependency_profiles_missing:{}", row.crate_name));
        }
        let mut profile_ids = BTreeSet::new();
        for profile in &row.profiles {
            if profile.adapter_id.is_empty()
                || !profile_ids.insert(&profile.adapter_id)
                || profile.timeout.detail().trim().is_empty()
                || profile.retry.detail().trim().is_empty()
                || profile.cache_invalidation_inputs.is_empty()
                || profile
                    .cache_invalidation_inputs
                    .iter()
                    .any(|s| s.trim().is_empty())
            {
                failures.push(format!(
                    "dependency_profile_incomplete:{}:{}",
                    row.crate_name, profile.adapter_id
                ));
            }
            failures.extend(crate::dependency_profiles::check(
                row, profile, reports, boundaries, test_only,
            ));
        }
    }
    for dependency in &metadata.dependencies {
        if !rows.iter().any(|row| {
            row.manifest == dependency.manifest && row.crate_name == dependency.crate_name
        }) {
            failures.push(format!(
                "dependency_unregistered:{}:{}:{}",
                dependency.manifest, dependency.crate_name, dependency.requirement
            ));
        }
    }
    for (path, report) in reports {
        if test_only.contains(path) {
            continue;
        }
        let manifest = owning_manifest(path, inventory, metadata);
        for usage in &report.dependencies {
            if !metadata
                .dependencies
                .iter()
                .any(|d| Some(d.manifest.as_str()) == manifest && d.crate_name == usage.crate_name)
            {
                continue;
            }
            let allowed = rows
                .iter()
                .filter(|r| {
                    Some(r.manifest.as_str()) == manifest && r.crate_name == usage.crate_name
                })
                .flat_map(|r| &r.profiles)
                .any(|profile| match &profile.contract {
                    ProfileContract::FunctionAdapters { module, functions } => {
                        module == path
                            && usage
                                .function
                                .as_ref()
                                .is_some_and(|f| functions.contains(f))
                    }
                    ProfileContract::PureOperations { module, symbols } => {
                        module == path
                            && symbols.contains(&usage.symbol)
                            && crate::pure::permitted(&usage.symbol)
                    }
                    ProfileContract::OwnedAdapter { module, .. } => module == path,
                    ProfileContract::TypedDeclarations { declarations } => {
                        declarations.iter().any(|site| {
                            &site.module == path
                                && usage
                                    .owner
                                    .as_ref()
                                    .is_some_and(|name| site.symbols.contains(name))
                        })
                    }
                });
            if !allowed {
                failures.push(format!(
                    "dependency_direct_bypass:{path}:{}:{}:{}",
                    usage.crate_name,
                    usage.symbol,
                    usage.owner.as_deref().unwrap_or("module")
                ));
            }
        }
    }
    failures
}

fn owning_manifest<'a>(
    path: &str,
    inventory: &'a Inventory,
    metadata: &'a Metadata,
) -> Option<&'a str> {
    inventory
        .files
        .keys()
        .chain(metadata.dependencies.iter().map(|d| &d.manifest))
        .filter(|p| p.ends_with("Cargo.toml"))
        .filter(|p| {
            let parent = p.strip_suffix("Cargo.toml").unwrap();
            path.starts_with(parent)
        })
        .max_by_key(|p| p.len())
        .map(String::as_str)
}

pub fn named_symbol(symbol: &str, reports: &BTreeMap<String, Report>) -> bool {
    if symbol.is_empty() || symbol.contains('*') || symbol.contains('?') {
        return false;
    }
    let Ok(parsed) = syn::parse_str::<syn::TypePath>(symbol) else {
        return false;
    };
    let last = parsed.path.segments.last().unwrap().ident.to_string();
    if matches!(
        last.as_str(),
        "String" | "Path" | "PathBuf" | "Value" | "Map" | "HashMap" | "BTreeMap" | "Vec" | "Option"
    ) {
        return false;
    }
    reports.values().any(|r| {
        r.types.iter().any(|t| t.name == last || t.name == symbol)
            || (parsed.path.segments.len() > 1 && r.dependencies.iter().any(|d| d.symbol == symbol))
            || r.functions
                .iter()
                .any(|f| f.resolved_error.as_deref() == Some(symbol))
    })
}

#[cfg(test)]
pub fn check(
    rows: &[DependencyRow],
    metadata: &Metadata,
    reports: &BTreeMap<String, Report>,
    inventory: &Inventory,
    boundaries: &[BoundaryRow],
) -> Vec<String> {
    check_with_scope(
        rows,
        metadata,
        reports,
        inventory,
        boundaries,
        &BTreeSet::new(),
    )
}
