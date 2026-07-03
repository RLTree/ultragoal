use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod graph;
mod graph_edges;
mod inventory;
mod inventory_requirements;
mod paths;
mod registry;
mod source;

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
    out.extend(source::source_text_failures(root));
    out.extend(inventory::generated_failures(root, &inventory));
    out
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
pub(crate) fn generated_boundary_failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    inventory::generated_failures(root, inventory)
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
