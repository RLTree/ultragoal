use serde_json::{Map, Value};
use std::path::Path;

pub(crate) const REQUIRED_SURFACES: &[&str] = &[
    "cli command families",
    "validator check families",
    "schema parse boundaries",
    "receipt proof artifacts",
    "fixture report artifacts",
    "package plugin resources",
    "source install cache registry surfaces",
    "claim guards update_goal eligibility",
    "local spool live exporters",
    "observability stack configs",
];

pub(super) fn check(root: &Path, value: &Value, out: &mut Vec<String>) {
    for surface in REQUIRED_SURFACES {
        if !surfaces_contain(value, surface) {
            out.push(format!("observability_surface_inventory_missing:{surface}"));
        }
    }
    reject_unknown_surface_rows(value, out);
    require_surface_inventory(root, value, out);
}

fn surfaces_contain(value: &Value, surface: &str) -> bool {
    value
        .get("surfaces")
        .and_then(Value::as_array)
        .is_some_and(|rows| rows.iter().any(|row| row.as_str() == Some(surface)))
}

fn reject_unknown_surface_rows(value: &Value, out: &mut Vec<String>) {
    for surface in value
        .get("surfaces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !REQUIRED_SURFACES.contains(&surface) {
            out.push(format!("observability_surface_inventory_unknown:{surface}"));
        }
    }
    if let Some(rows) = value.get("surface_inventory").and_then(Value::as_object) {
        for surface in rows.keys() {
            if !REQUIRED_SURFACES.contains(&surface.as_str()) {
                out.push(format!("observability_surface_fitting_unknown:{surface}"));
            }
        }
    }
}

fn require_surface_inventory(root: &Path, value: &Value, out: &mut Vec<String>) {
    let Some(rows) = value.get("surface_inventory").and_then(Value::as_object) else {
        out.push("observability_surface_fitting_inventory_missing".to_string());
        return;
    };
    for surface in REQUIRED_SURFACES {
        let Some(row) = rows.get(*surface) else {
            out.push(format!("observability_surface_fitting_missing:{surface}"));
            continue;
        };
        let Some(object) = row.as_object() else {
            out.push(format!(
                "observability_surface_fitting_row_not_object:{surface}"
            ));
            continue;
        };
        require_surface_row(root, surface, object, out);
    }
}

fn require_surface_row(
    root: &Path,
    surface: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    match row.get("fitting_status").and_then(Value::as_str) {
        Some("fitted") => require_fitted_evidence(root, surface, row, out),
        Some(status @ ("partially_fitted" | "unfitted")) => {
            require_unfitted_metadata(surface, row, out);
            out.push(format!(
                "observability_surface_fitting_unfitted:{surface}:{status}"
            ));
        }
        Some(other) => out.push(format!(
            "observability_surface_fitting_status_invalid:{surface}:{other}"
        )),
        None => out.push(format!(
            "observability_surface_fitting_status_missing:{surface}"
        )),
    }
}

fn require_unfitted_metadata(surface: &str, row: &Map<String, Value>, out: &mut Vec<String>) {
    if !owner_tracking(row)
        || !typed_row_accounting(row)
        || !super::row_contract::complete(row)
        || !non_empty_array(row, "missing_surfaces")
        || !non_empty_string(row, "claim_impact")
    {
        out.push(format!(
            "observability_surface_fitting_missing_metadata:{surface}"
        ));
    }
}

fn typed_row_accounting(row: &Map<String, Value>) -> bool {
    row.get("fitted_surfaces").is_some_and(Value::is_array)
        && non_empty_string(row, "validator_check_id")
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

fn require_fitted_evidence(
    root: &Path,
    surface: &str,
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
        out.push(format!(
            "observability_surface_fitting_row_shape_only:{surface}"
        ));
        return;
    }
    super::proof::require_current_surface_receipts(root, surface, row, out);
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
