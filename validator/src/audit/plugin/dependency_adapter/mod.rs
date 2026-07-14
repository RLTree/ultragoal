mod cargo_manifest;
mod model;
mod policy;
mod registry;

use std::path::Path;

const REGISTRY_PATH: &str = "docs/dependency-adapter-registry.json";

pub(crate) fn failures(root: &Path) -> Vec<String> {
    let inventory = crate::audit::source_governance::audit(root).inventory;
    let dependencies = match cargo_manifest::load(&root.join("validator/Cargo.toml")) {
        Ok(value) => value,
        Err(failure) => return vec![failure],
    };
    let mut failures = Vec::new();
    let registry = match registry::load(&root.join(REGISTRY_PATH)) {
        Ok(value) => value,
        Err(failure) => {
            failures.push(failure);
            model::DependencyRegistry {
                schema_version: "dependency-adapter-registry-v2".to_string(),
                rows: Vec::new(),
            }
        }
    };
    failures.extend(policy::failures(&dependencies, &registry, &inventory));
    failures.sort();
    failures.dedup();
    failures
}

#[cfg(test)]
pub(crate) use model::{
    BoundaryKind, DeclarationSite, DependencyProfile, DependencyRegistry, DependencyRow,
    DirectDependency, OperationalPolicy, ProfileContract,
};
#[cfg(test)]
pub(crate) use policy::failures as failures_for_test;
