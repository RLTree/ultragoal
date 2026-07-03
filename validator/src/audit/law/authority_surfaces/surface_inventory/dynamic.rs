use super::AuthoritySurfaceInventoryRow;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const FOUNDATIONAL_LAW_RED_FIXTURE_PREFIXES: &[&str] = &[
    "authority-source-binding",
    "typed-records-over-prose",
    "generated-proof-artifact-provenance-anti-fabrication",
    "distinct-proof-surfaces-claim-ceilings",
    "total-authority-types-impossible-state-elimination",
    "raw-private-artifact-handling-category-only-evidence",
    "validator-theater-miswire-resistance",
    "namespace-progressive-disclosure",
    "semantic-domain-type-naming",
];

pub(super) fn rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut rows = Vec::new();
    rows.extend(foundational_red_fixture_rows(root, inventory));
    rows.extend(receipt_schema_rows(root, inventory));
    rows.extend(generated_artifact_rows(root, inventory));
    rows
}

fn foundational_red_fixture_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let catalog = crate::json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .unwrap_or(Value::Null);
    catalog
        .as_array()
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(|row| {
            let id = row.get("id").and_then(Value::as_str)?;
            if !FOUNDATIONAL_LAW_RED_FIXTURE_PREFIXES
                .iter()
                .any(|prefix| id.starts_with(prefix))
            {
                return None;
            }
            row.get("packet_path")
                .and_then(Value::as_str)
                .map(|path| authority_row("red_fixture", path, true, root, inventory))
        })
        .collect()
}

fn generated_artifact_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_json_artifacts(root, &root.join("docs/generated"), &mut out);
    collect_json_artifacts(root, &root.join("examples/generated"), &mut out);
    out.into_iter()
        .map(|path| authority_row("generated_artifact", &path, true, root, inventory))
        .collect()
}

fn receipt_schema_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_receipt_schemas(root, &root.join("schemas"), &mut out);
    out.into_iter()
        .map(|path| authority_row("receipt_schema", &path, true, root, inventory))
        .collect()
}

fn collect_receipt_schemas(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_receipt_schemas(root, &path, out);
            continue;
        }
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();
        if (file_name.contains("receipt") || file_name.contains("proof"))
            && path.extension().and_then(|ext| ext.to_str()) == Some("json")
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn collect_json_artifacts(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_artifacts(root, &path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json")
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn authority_row(
    role: &str,
    path: &str,
    package_inventory_required: bool,
    root: &Path,
    inventory: &BTreeSet<String>,
) -> AuthoritySurfaceInventoryRow {
    AuthoritySurfaceInventoryRow {
        role: role.to_string(),
        path: path.to_string(),
        package_inventory_required,
        exists_on_disk: root.join(path).is_file(),
        listed_in_package_inventory: inventory.contains(path),
    }
}
