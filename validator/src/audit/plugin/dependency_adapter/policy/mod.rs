mod profiles;
mod registry_rows;
mod reports;

use super::model::{DependencyRegistry, DirectDependency};
use crate::audit::source_governance::GovernedInventory;
use std::collections::BTreeMap;

pub(crate) fn failures(
    dependencies: &[DirectDependency],
    registry: &DependencyRegistry,
    inventory: &GovernedInventory,
) -> Vec<String> {
    let mut failures = Vec::new();
    if registry.schema_version != "dependency-adapter-registry-v2" {
        failures.push(format!(
            "dependency_adapter_registry_schema_unknown:{}",
            registry.schema_version
        ));
    }
    let dependency_map = dependencies
        .iter()
        .map(|dependency| (dependency.crate_name.as_str(), dependency))
        .collect::<BTreeMap<_, _>>();
    let mut rows = BTreeMap::new();
    for row in &registry.rows {
        if rows.insert(row.crate_name.as_str(), row).is_some() {
            failures.push(format!(
                "dependency_adapter_registry_duplicate:{}",
                row.crate_name
            ));
        }
        failures.extend(registry_rows::failures(row, &dependency_map));
    }
    let reports = reports::syntax_reports(inventory, &mut failures);
    for dependency in dependencies {
        let Some(row) = rows.get(dependency.crate_name.as_str()) else {
            failures.push(format!(
                "dependency_adapter_registry_missing:{}",
                dependency.crate_name
            ));
            for path in reports::observed_sites(dependency, &reports) {
                failures.push(format!(
                    "dependency_adapter_unregistered_call_site:{}:{path}",
                    dependency.crate_name
                ));
            }
            continue;
        };
        failures.extend(profiles::failures(dependency, row, &reports));
    }
    failures.sort();
    failures.dedup();
    failures
}
