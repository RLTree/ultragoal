use serde_json::Value;
use std::path::Path;

pub(super) fn complete(root: &Path) -> Result<(), String> {
    let failures = crate::audit::observability::command_inventory_failures(root);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failure_summary(root, &failures))
    }
}

pub(super) fn failure_summary(root: &Path, failures: &[String]) -> String {
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .unwrap_or(Value::Null);
    let board = inventory
        .get("observability_control_board")
        .unwrap_or(&Value::Null);
    let first_incomplete = board.get("first_incomplete").unwrap_or(&Value::Null);
    format!(
        "observability command inventory incomplete: status={} total_failures={} first_failure={} control_board_first_family={} control_board_first_incomplete={} control_board_first_status={} next_unobservable_surface={} family_counts={}",
        text_field(board, "status", "unknown"),
        failures.len(),
        failures.first().map(String::as_str).unwrap_or("none"),
        text_field(first_incomplete, "family", "unknown"),
        text_field(first_incomplete, "id", "unknown"),
        text_field(first_incomplete, "observability_status", "unknown"),
        text_field(first_incomplete, "next_unobservable_surface", "unknown"),
        family_counts(board)
    )
}

pub(super) fn family_counts(board: &Value) -> String {
    let Some(families) = board.get("families").and_then(Value::as_object) else {
        return "unavailable".to_string();
    };
    let mut rows = families
        .iter()
        .map(|(family, row)| {
            format!(
                "{}={}/{}/{}/{}",
                family,
                count_field(row, "total"),
                count_field(row, "observable"),
                count_field(row, "partially_observable"),
                count_field(row, "unobservable")
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows.join(",")
}

fn count_field(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}

fn text_field<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}
