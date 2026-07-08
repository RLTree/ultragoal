use super::row;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(crate) struct InventoryArtifactBinding {
    pub(crate) path: String,
    pub(crate) digest: String,
    pub(crate) status: String,
}

pub(super) fn full(rows: Vec<row::PackageSurfaceRow>) -> Value {
    let mut kind_counts = BTreeMap::new();
    let mut authority_counts = BTreeMap::new();
    for surface in &rows {
        *kind_counts
            .entry(surface.surface_kind.clone())
            .or_insert(0usize) += 1;
        *authority_counts
            .entry(surface.authority_level.clone())
            .or_insert(0usize) += 1;
    }
    json!({
        "schema": "harness-ultragoal.package-surface-inventory.v1",
        "surface_count": rows.len(),
        "kind_counts": kind_counts,
        "authority_level_counts": authority_counts,
        "rows": rows.into_iter().map(row::value).collect::<Vec<_>>()
    })
}

pub(super) fn summary(
    rows: Vec<row::PackageSurfaceRow>,
    artifact: &InventoryArtifactBinding,
) -> Value {
    let row_count = rows.len();
    let mut kind_counts = BTreeMap::new();
    let mut authority_counts = BTreeMap::new();
    let row_values = rows.into_iter().map(row::value).collect::<Vec<_>>();
    for row in &row_values {
        if let Some(kind) = row.get("surface_kind").and_then(Value::as_str) {
            *kind_counts.entry(kind.to_string()).or_insert(0usize) += 1;
        }
        if let Some(level) = row.get("authority_level").and_then(Value::as_str) {
            *authority_counts.entry(level.to_string()).or_insert(0usize) += 1;
        }
    }
    json!({
        "schema": "harness-ultragoal.package-surface-inventory-summary.v1",
        "surface_count": row_count,
        "kind_counts": kind_counts,
        "authority_level_counts": authority_counts,
        "rows_digest": crate::digest::canonical_json(&Value::Array(row_values)),
        "rows_omitted_from_receipt": true,
        "row_materialization": materialization(artifact)
    })
}

fn materialization(binding: &InventoryArtifactBinding) -> Value {
    json!({
        "artifact_path": binding.path,
        "artifact_digest": binding.digest,
        "artifact_status": binding.status,
        "artifact_role": "governed package-surface inventory artifact",
        "claim_ceiling": "source_local_package_surface_inventory_only"
    })
}
