use std::path::Path;

pub(super) fn all(root: &Path) -> Vec<String> {
    let store = crate::schema_catalog::load(root);
    let mut out = catalog_failures(&store);
    out.extend(prefixed(
        "plugin_self_law",
        crate::audit::plugin::laws::package_failures(root, &store),
    ));
    out.extend(prefixed(
        "cli_performance",
        crate::audit::cli::performance::package_failures(root),
    ));
    out.extend(prefixed(
        "final_packet",
        crate::audit::final_packet::package_failures(root, &store),
    ));
    out.extend(super::red::report_failures(root, &store));
    out
}

pub(super) fn registry_surface(root: &Path) -> Vec<String> {
    let store = crate::schema_catalog::load(root);
    let mut out = catalog_failures(&store);
    out.extend(prefixed(
        "registry_surface",
        crate::audit::plugin::registry::failures(root, &store),
    ));
    out
}

fn catalog_failures(store: &crate::schema_catalog::SchemaStore) -> Vec<String> {
    store
        .errors
        .iter()
        .map(|failure| format!("schema_catalog:{failure}"))
        .collect()
}

fn prefixed(prefix: &str, failures: Vec<String>) -> Vec<String> {
    failures
        .into_iter()
        .map(|failure| format!("{prefix}:{failure}"))
        .collect()
}
