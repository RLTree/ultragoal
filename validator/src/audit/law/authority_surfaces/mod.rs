use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod inventory;
mod registry;
mod source;
mod surface_registry;

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
    out.extend(surface_registry::failures(root, &inventory));
    out.extend(registry::law_registry_failures(
        root,
        &laws,
        &registry::red_catalog_ids(&red_catalog),
        &inventory,
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
pub(crate) fn generated_boundary_failures_for_test(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    inventory::generated_failures(root, inventory)
}
