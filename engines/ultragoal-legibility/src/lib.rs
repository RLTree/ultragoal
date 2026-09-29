#[cfg(test)]
mod adversarial_tests;
mod audit;
mod boundaries;
#[cfg(test)]
mod boundary_tests;
mod conditional;
mod coverage;
mod dependencies;
mod dependency_profiles;
mod diagnostics;
mod domain_response;
mod generic_domains;
mod inclusions;
mod inventory;
mod metadata;
mod model;
mod modules;
mod outputs;
#[cfg(test)]
mod portable_tests;
mod pure;
mod pure_syntax;
mod registry;
mod registry_types;
mod source_context;
mod syntax;
#[cfg(test)]
mod tests;

/// Audit an explicit project and exact relative registry without approving findings.
pub fn run(root: &std::path::Path, registry_path: &str, inventory: bool) -> serde_json::Value {
    audit::run(root, registry_path, inventory)
}
