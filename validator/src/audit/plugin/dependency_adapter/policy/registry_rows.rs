use super::super::model::{DependencyRow, DirectDependency};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn failures(
    row: &DependencyRow,
    dependencies: &BTreeMap<&str, &DirectDependency>,
) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(dependency) = dependencies.get(row.crate_name.as_str()) else {
        return vec![format!(
            "dependency_adapter_registry_stale:{}",
            row.crate_name
        )];
    };
    if dependency.version_requirement != row.version_requirement {
        failures.push(format!(
            "dependency_adapter_registry_version_mismatch:{}:{}:{}",
            row.crate_name, dependency.version_requirement, row.version_requirement
        ));
    }
    if row.owner.trim().len() < 3 || row.purpose.trim().len() < 16 {
        failures.push(format!(
            "dependency_adapter_registry_ownership_missing:{}",
            row.crate_name
        ));
    }
    if !row.upstream_docs.starts_with("https://")
        || !row
            .upstream_docs
            .to_ascii_lowercase()
            .contains(&row.crate_name.replace('_', "-"))
    {
        failures.push(format!(
            "dependency_adapter_registry_upstream_docs_invalid:{}",
            row.crate_name
        ));
    }
    if row.profiles.is_empty() {
        failures.push(format!(
            "dependency_adapter_registry_profiles_missing:{}",
            row.crate_name
        ));
    }
    let mut identifiers = BTreeSet::new();
    let mut modules = BTreeMap::<&str, Vec<&str>>::new();
    for profile in &row.profiles {
        if !semantic_id(&profile.adapter_id) || !identifiers.insert(profile.adapter_id.as_str()) {
            failures.push(format!(
                "dependency_adapter_profile_id_invalid:{}:{}",
                row.crate_name, profile.adapter_id
            ));
        }
        for module in profile.contract.modules() {
            modules.entry(module).or_default().push(&profile.adapter_id);
        }
    }
    for (module, profiles) in modules {
        if profiles.len() > 1 {
            failures.push(format!(
                "dependency_adapter_profile_module_ambiguous:{}:{module}:{}",
                row.crate_name,
                profiles.join(",")
            ));
        }
    }
    failures
}

fn semantic_id(value: &str) -> bool {
    !value.is_empty()
        && value != "adapter"
        && value != "shared"
        && value != "common"
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
}
