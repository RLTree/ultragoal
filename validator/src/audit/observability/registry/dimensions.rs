use serde_json::{Map, Value};
use std::path::Path;

use super::dimension_ids::{InventoryFamily, inventory_families};

pub(super) fn check(root: &Path, value: &Value, out: &mut Vec<String>) {
    for family in inventory_families() {
        require_named_rows(root, value, family, out);
    }
}

fn require_named_rows(root: &Path, value: &Value, family: InventoryFamily, out: &mut Vec<String>) {
    for id in family.ids {
        if !list_contains(value, family.list_key, id) {
            out.push(format!("{}_inventory_missing:{id}", family.failure_prefix));
        }
    }
    reject_unknown_rows(value, family, out);
    let Some(rows) = value.get(family.inventory_key).and_then(Value::as_object) else {
        out.push(format!(
            "{}_fitting_inventory_missing",
            family.failure_prefix
        ));
        return;
    };
    for id in family.ids {
        let Some(row) = rows.get(*id) else {
            out.push(format!("{}_fitting_missing:{id}", family.failure_prefix));
            continue;
        };
        let Some(object) = row.as_object() else {
            out.push(format!(
                "{}_fitting_row_not_object:{id}",
                family.failure_prefix
            ));
            continue;
        };
        require_row(root, family.failure_prefix, id, object, out);
    }
}

fn reject_unknown_rows(value: &Value, family: InventoryFamily, out: &mut Vec<String>) {
    for id in value
        .get(family.list_key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !family.ids.contains(&id) {
            out.push(format!("{}_inventory_unknown:{id}", family.failure_prefix));
        }
    }
    if let Some(rows) = value.get(family.inventory_key).and_then(Value::as_object) {
        for id in rows.keys() {
            if !family.ids.contains(&id.as_str()) {
                out.push(format!("{}_fitting_unknown:{id}", family.failure_prefix));
            }
        }
    }
}

fn require_row(
    root: &Path,
    prefix: &str,
    id: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    match row.get("fitting_status").and_then(Value::as_str) {
        Some("fitted") => require_fitted_evidence(root, prefix, id, row, out),
        Some(status @ ("partially_fitted" | "unfitted")) => {
            require_unfitted_metadata(prefix, id, row, out);
            out.push(format!("{prefix}_fitting_unfitted:{id}:{status}"));
        }
        Some(other) => out.push(format!("{prefix}_fitting_status_invalid:{id}:{other}")),
        None => out.push(format!("{prefix}_fitting_status_missing:{id}")),
    }
}

fn require_unfitted_metadata(
    prefix: &str,
    id: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    if !typed_row_accounting(row)
        || !super::row_contract::complete(row)
        || !non_empty_array(row, "missing_surfaces")
        || !non_empty_string(row, "claim_impact")
        || !owner_tracking(row)
    {
        out.push(format!("{prefix}_fitting_missing_metadata:{id}"));
    }
}

fn require_fitted_evidence(
    root: &Path,
    prefix: &str,
    id: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    let required = [
        non_empty_array(row, "fitted_surfaces"),
        empty_array(row, "missing_surfaces"),
        non_empty_string(row, "operation"),
        non_empty_string(row, "validator_check_id"),
        non_empty_array(row, "focused_tests"),
        non_empty_array(row, "receipt_paths"),
        non_empty_array(row, "live_query_proof_paths"),
        non_empty_array(row, "same_candidate_query_proof_paths"),
        owner_tracking(row),
        super::row_contract::fitted(row),
        non_empty_string(row, "claim_impact"),
    ];
    if required.into_iter().any(|ok| !ok) {
        out.push(format!("{prefix}_fitting_row_shape_only:{id}"));
        return;
    }
    super::proof::require_current_surface_receipts(root, id, row, out);
}

fn typed_row_accounting(row: &Map<String, Value>) -> bool {
    non_empty_string(row, "operation")
        && non_empty_string(row, "validator_check_id")
        && row.get("fitted_surfaces").is_some_and(Value::is_array)
        && row.get("focused_tests").is_some_and(Value::is_array)
        && row.get("receipt_paths").is_some_and(Value::is_array)
        && row
            .get("live_query_proof_paths")
            .is_some_and(Value::is_array)
        && row
            .get("same_candidate_query_proof_paths")
            .is_some_and(Value::is_array)
        && row.get("red_fixtures").is_some_and(Value::is_array)
        && row.get("green_fixtures").is_some_and(Value::is_array)
        && row.get("tamper_fixtures").is_some_and(Value::is_array)
}

fn list_contains(value: &Value, key: &str, id: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|rows| rows.iter().any(|row| row.as_str() == Some(id)))
}

fn non_empty_string(row: &Map<String, Value>, key: &str) -> bool {
    row.get(key)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn non_empty_array(row: &Map<String, Value>, key: &str) -> bool {
    row.get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

fn empty_array(row: &Map<String, Value>, key: &str) -> bool {
    row.get(key)
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
}

fn owner_tracking(row: &Map<String, Value>) -> bool {
    non_empty_string(row, "current_owner_surface") && non_empty_string(row, "next_unfitted_surface")
}
