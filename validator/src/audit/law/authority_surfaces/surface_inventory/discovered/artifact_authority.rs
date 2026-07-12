use super::{AuthoritySurfaceInventoryRow, authority_row, collect_files};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut rows = Vec::new();
    rows.extend(receipt_schema_rows(root, inventory));
    rows.extend(runtime_receipt_rows(root, inventory));
    rows.extend(generated_artifact_rows(root, inventory));
    rows.extend(package_inventory_rows(root, inventory));
    rows
}

fn receipt_schema_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("schemas"), &mut out);
    out.into_iter()
        .filter(|path| {
            path.ends_with(".json")
                && path
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name.contains("receipt") || name.contains("proof"))
        })
        .map(|path| authority_row("receipt_schema", &path, true, root, inventory))
        .collect()
}

fn runtime_receipt_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("validation_artifacts"), &mut out);
    out.into_iter()
        .filter(|path| path.ends_with(".json"))
        .filter(|path| !inventory_materialization_artifact(path))
        .map(|path| authority_row("runtime_receipt", &path, false, root, inventory))
        .collect()
}

fn inventory_materialization_artifact(path: &str) -> bool {
    matches!(
        path,
        "validation_artifacts/package/foundational-surface-inventory.json"
            | "validation_artifacts/package/package-surface-inventory.json"
    )
}

fn generated_artifact_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_json_artifacts(root, &root.join("docs/generated"), &mut out);
    collect_json_artifacts(root, &root.join("examples/generated"), &mut out);
    out.into_iter()
        .filter(|path| path != super::super::super::inventory::DEAUTHORIZED_COMMAND_INVENTORY)
        .map(|path| authority_row("generated_artifact", &path, true, root, inventory))
        .collect()
}

fn package_inventory_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    [
        "plugin-manifest-draft.json",
        "docs/plugin-cohesion-manifest.json",
        ".codex-plugin/plugin.json",
    ]
    .into_iter()
    .map(|path| authority_row("package_inventory", path, true, root, inventory))
    .collect()
}

fn collect_json_artifacts(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let mut files = Vec::new();
    collect_files(root, dir, &mut files);
    out.extend(files.into_iter().filter(|path| path.ends_with(".json")));
}
