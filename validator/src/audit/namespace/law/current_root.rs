use super::{
    CLASS_REGISTRY_PATH, binding_failures, class_registry_value_failures, path_name_failures,
    plugin_interfaces,
};
use crate::json_boundary;
use std::path::Path;

pub fn failures(root: &Path) -> Vec<String> {
    let governed = crate::audit::source_governance::capture_namespace_sources(root);
    let source_paths = governed
        .inventory
        .sources
        .iter()
        .map(|source| source.relative.clone())
        .collect::<Vec<_>>();
    let rust_paths = crate::audit::namespace::source::topology::repo_source_paths_from_actual_files(
        &source_paths,
    );
    let actual = crate::package::inventory::closure::actual_files(root).unwrap_or_default();
    let mut out = governed.failures;
    out.extend(path_name_failures(&rust_paths));
    out.extend(plugin_interfaces::failures(&actual));
    out.extend(
        crate::audit::namespace::source::topology::failures_with_repo_paths(&[], &rust_paths),
    );
    out.extend(
        crate::audit::namespace::source::identifiers::failures_for_inventory(&governed.inventory),
    );
    out.extend(crate::audit::namespace::classes::legacy_surface_failures(
        root, &actual,
    ));
    match json_boundary::read_json(&root.join(CLASS_REGISTRY_PATH)) {
        Ok(registry) => {
            out.extend(class_registry_value_failures(root, &registry));
            out.extend(crate::audit::namespace::classes::resolution_failures(
                &registry,
                &rust_paths,
            ));
        }
        Err(error) => out.push(format!("namespace_class_registry_file_missing:{error}")),
    }
    out.extend(binding_failures(root));
    out.sort();
    out.dedup();
    out
}
