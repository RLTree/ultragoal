use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod authority_labels;
mod graph;
mod graph_edges;
mod inventory;
#[path = "inventory/required_surfaces.rs"]
mod inventory_required_surfaces;
#[path = "inventory/requirements.rs"]
mod inventory_requirements;
mod package_surfaces;
mod paths;
mod registry;
mod source;
mod surface_inventory;

pub(crate) use package_surfaces::InventoryArtifactBinding as PackageSurfaceArtifactBinding;
#[cfg(test)]
pub(crate) use source::BoundaryRow;
#[cfg(test)]
pub(crate) use source::failures_for_sources_and_rows;
pub(crate) use surface_inventory::InventoryArtifactBinding as FoundationalSurfaceArtifactBinding;

pub(crate) fn package_failures(root: &Path) -> Vec<(String, String)> {
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .unwrap_or(Value::Null);
    let inventory = crate::package::inventory::inventory_paths(&manifest)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let laws = crate::json_boundary::read_json(&root.join("docs/mandatory-law-surfaces.json"))
        .unwrap_or(Value::Null);
    let red_catalog = crate::json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .unwrap_or(Value::Null);
    let mut out = Vec::new();
    out.extend(inventory_requirements::failures(root, &inventory));
    out.extend(registry::law_registry_failures(
        root,
        &laws,
        &registry::red_catalog_ids(&red_catalog),
        &inventory,
    ));
    out.extend(graph::failures(
        root,
        &inventory,
        &registry::red_catalog_ids(&red_catalog),
    ));
    out.extend(authority_labels::package_json_label_failures(
        root, &inventory,
    ));
    out.extend(package_surfaces::failures(root, &inventory));
    out.extend(source::source_text_failures(root));
    out.extend(inventory::generated_failures(root, &inventory));
    out
}

#[cfg(test)]
pub(crate) fn foundational_surface_inventory(root: &Path) -> Value {
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .unwrap_or(Value::Null);
    let inventory = crate::package::inventory::inventory_paths(&manifest)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut value = surface_inventory::value(root, &inventory);
    value["package_surface_inventory"] = package_surfaces::value(root, &inventory);
    value
}

pub(crate) fn foundational_surface_inventory_with_artifacts(
    root: &Path,
    foundational_artifact: &FoundationalSurfaceArtifactBinding,
    package_artifact: &PackageSurfaceArtifactBinding,
) -> Value {
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .unwrap_or(Value::Null);
    let inventory = crate::package::inventory::inventory_paths(&manifest)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut value = surface_inventory::summary_value(root, &inventory, foundational_artifact);
    value["package_surface_inventory"] =
        package_surfaces::summary_value(root, &inventory, package_artifact);
    value
}

pub(crate) fn foundational_surface_inventory_artifact(root: &Path) -> Value {
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .unwrap_or(Value::Null);
    let inventory = crate::package::inventory::inventory_paths(&manifest)
        .into_iter()
        .collect::<BTreeSet<_>>();
    surface_inventory::value(root, &inventory)
}

pub(crate) fn package_surface_inventory_artifact(root: &Path) -> Value {
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .unwrap_or(Value::Null);
    let inventory = crate::package::inventory::inventory_paths(&manifest)
        .into_iter()
        .collect::<BTreeSet<_>>();
    package_surfaces::value(root, &inventory)
}

#[cfg(test)]
pub(crate) fn raw_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    source::raw_authority_failures_for_test(rel, text)
}

#[cfg(test)]
pub(crate) fn output_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    source::output_authority_failures_for_test(rel, text)
}

#[cfg(test)]
pub(crate) fn source_text_failures_for_test(root: &Path) -> Vec<(String, String)> {
    source::source_text_failures(root)
}

#[cfg(test)]
pub(crate) fn package_json_label_failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    authority_labels::package_json_label_failures(root, inventory)
}

#[cfg(test)]
pub(crate) fn generated_boundary_failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    inventory::generated_failures(root, inventory)
}

#[cfg(test)]
pub(crate) fn package_surface_failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    package_surfaces::failures_for_test(root, inventory)
}

#[cfg(test)]
pub(crate) fn required_surfaces_for_test() -> Vec<(&'static str, &'static str, bool)> {
    inventory_requirements::required_surfaces_for_test()
}

#[cfg(test)]
pub(crate) fn required_surface_failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    inventory_requirements::failures(root, inventory)
}

#[cfg(test)]
pub(crate) fn registry_law_failures_for_test(
    root: &Path,
    registry: &Value,
    red_ids: &BTreeSet<String>,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    registry::law_registry_failures(root, registry, red_ids, inventory)
}

#[cfg(test)]
pub(crate) fn authority_graph_failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
    mandatory: &Value,
    obligations: &Value,
    trace: &Value,
    standards: &Value,
    standards_audit: &str,
    red_ids: &BTreeSet<String>,
) -> Vec<(String, String)> {
    graph::authority_graph_failures(
        root,
        inventory,
        mandatory,
        obligations,
        trace,
        standards,
        standards_audit,
        red_ids,
    )
}
