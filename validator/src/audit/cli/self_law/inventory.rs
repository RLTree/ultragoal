use std::collections::BTreeMap;
use std::path::Path;

pub(in crate::audit) fn append(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    match crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json")) {
        Ok(manifest) => {
            for failure in crate::package::inventory::inventory_closure_failures(root, &manifest) {
                let check = if failure.contains("duplicates=") {
                    "plugin-inventory-exactly-once"
                } else {
                    "plugin-inventory-closure"
                };
                push(failures, check, failure);
            }
            for failure in crate::skill_links::manifest_failures(root, &manifest) {
                push(failures, "skill-inventory-closure", failure.detail);
            }
            for failure in crate::package::resource::purpose::failures(root, &manifest) {
                push(
                    failures,
                    "plugin-inventory-closure",
                    format!("{}: {}", failure.code, failure.detail),
                );
            }
            for failure in crate::audit::namespace::law::package_failures(root, &manifest) {
                push(failures, "namespace-progressive-disclosure", failure);
            }
        }
        Err(error) => push(
            failures,
            "plugin-inventory-closure",
            format!("plugin-manifest-draft.json load failed: {error}"),
        ),
    }
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, check: &str, detail: impl Into<String>) {
    failures
        .entry(check.to_string())
        .or_default()
        .push(detail.into());
}
