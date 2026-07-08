use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod discovered;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthoritySurfaceInventoryRow {
    pub(crate) role: String,
    pub(crate) path: String,
    pub(crate) package_inventory_required: bool,
    pub(crate) existence_required: bool,
    pub(crate) exists_on_disk: bool,
    pub(crate) listed_in_package_inventory: bool,
}

pub(crate) struct InventoryArtifactBinding {
    pub(crate) path: String,
    pub(crate) digest: String,
    pub(crate) status: String,
}

pub(super) fn rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut rows = super::inventory_requirements::required_surfaces()
        .iter()
        .map(|surface| AuthoritySurfaceInventoryRow {
            role: surface.role.to_string(),
            path: surface.rel.to_string(),
            package_inventory_required: surface.package_inventory_required,
            existence_required: surface.existence_required,
            exists_on_disk: root.join(surface.rel).is_file(),
            listed_in_package_inventory: inventory.contains(surface.rel),
        })
        .collect::<Vec<_>>();
    rows.extend(discovered::rows(root, inventory));
    rows.sort_by(|left, right| left.role.cmp(&right.role).then(left.path.cmp(&right.path)));
    rows.dedup_by(|left, right| left.role == right.role && left.path == right.path);
    rows
}

pub(super) fn value(root: &Path, inventory: &BTreeSet<String>) -> Value {
    let rows = rows(root, inventory);
    full_value(rows)
}

pub(super) fn summary_value(
    root: &Path,
    inventory: &BTreeSet<String>,
    artifact: &InventoryArtifactBinding,
) -> Value {
    let rows = rows(root, inventory);
    let surface_count = rows.len();
    let mut role_counts = BTreeMap::new();
    for row in &rows {
        *role_counts.entry(row.role.clone()).or_insert(0usize) += 1;
    }
    let missing_surface_count = rows
        .iter()
        .filter(|row| row.existence_required && !row.exists_on_disk)
        .count();
    let package_inventory_missing_count = rows
        .iter()
        .filter(|row| row.package_inventory_required && !row.listed_in_package_inventory)
        .count();
    let row_values = rows.into_iter().map(row_value).collect::<Vec<_>>();
    json!({
        "schema": "harness-ultragoal.foundational-law-surface-inventory-summary.v1",
        "surface_count": surface_count,
        "missing_surface_count": missing_surface_count,
        "package_inventory_missing_count": package_inventory_missing_count,
        "role_counts": role_counts,
        "rows_digest": crate::digest::canonical_json(&Value::Array(row_values)),
        "rows_omitted_from_receipt": true,
        "row_materialization": json!({
            "artifact_path": artifact.path,
            "artifact_digest": artifact.digest,
            "artifact_status": artifact.status,
            "artifact_role": "governed foundational-law surface inventory artifact",
            "claim_ceiling": "source_local_foundational_surface_inventory_only"
        })
    })
}

fn full_value(rows: Vec<AuthoritySurfaceInventoryRow>) -> Value {
    let mut role_counts = BTreeMap::new();
    for row in &rows {
        *role_counts.entry(row.role.clone()).or_insert(0usize) += 1;
    }
    let missing_surface_count = rows
        .iter()
        .filter(|row| row.existence_required && !row.exists_on_disk)
        .count();
    let package_inventory_missing_count = rows
        .iter()
        .filter(|row| row.package_inventory_required && !row.listed_in_package_inventory)
        .count();
    json!({
        "schema": "harness-ultragoal.foundational-law-surface-inventory.v1",
        "surface_count": rows.len(),
        "missing_surface_count": missing_surface_count,
        "package_inventory_missing_count": package_inventory_missing_count,
        "role_counts": role_counts,
        "rows": rows.into_iter().map(row_value).collect::<Vec<_>>()
    })
}

fn row_value(row: AuthoritySurfaceInventoryRow) -> Value {
    json!({
        "role": row.role,
        "path": row.path,
        "package_inventory_required": row.package_inventory_required,
        "existence_required": row.existence_required,
        "exists_on_disk": row.exists_on_disk,
        "listed_in_package_inventory": row.listed_in_package_inventory,
        "surface_state": surface_state(&row)
    })
}

fn surface_state(row: &AuthoritySurfaceInventoryRow) -> &'static str {
    if (!row.existence_required || row.exists_on_disk)
        && (!row.package_inventory_required || row.listed_in_package_inventory)
    {
        "available"
    } else {
        "blocked"
    }
}
