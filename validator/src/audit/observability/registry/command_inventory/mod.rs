use serde_json::{Map, Value};
use std::path::Path;

mod commands;
mod requirements;
pub(crate) use commands::REQUIRED_COMMANDS;

pub(super) fn check(root: &Path, value: &Value, out: &mut Vec<String>) {
    for command in REQUIRED_COMMANDS {
        if !commands_contain(value, command) {
            out.push(format!("observability_command_inventory_missing:{command}"));
        }
    }
    reject_unknown_inventory_commands(value, out);
    for key in requirements::row_requirement_keys() {
        if value.pointer(&format!("/row_requirements/{key}")) != Some(&Value::Bool(true)) {
            out.push(format!(
                "observability_command_inventory_requirement_missing:{key}"
            ));
        }
    }
    require_command_inventory_rows(root, value, out);
}

fn commands_contain(value: &Value, command: &str) -> bool {
    value
        .get("commands")
        .and_then(Value::as_array)
        .is_some_and(|rows| rows.iter().any(|row| row.as_str() == Some(command)))
}

fn reject_unknown_inventory_commands(value: &Value, out: &mut Vec<String>) {
    for command in value
        .get("commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !REQUIRED_COMMANDS.contains(&command) {
            out.push(format!("observability_command_inventory_unknown:{command}"));
        }
    }
    if let Some(rows) = value
        .get("command_observability_inventory")
        .and_then(Value::as_object)
    {
        for command in rows.keys() {
            if !REQUIRED_COMMANDS.contains(&command.as_str()) {
                out.push(format!("observability_command_telemetry_unknown:{command}"));
            }
        }
    }
}

fn require_command_inventory_rows(root: &Path, value: &Value, out: &mut Vec<String>) {
    let Some(rows) = value
        .get("command_observability_inventory")
        .and_then(Value::as_object)
    else {
        out.push("observability_command_observability_inventory_missing".to_string());
        return;
    };
    for command in REQUIRED_COMMANDS {
        let Some(row) = rows.get(*command) else {
            out.push(format!("observability_command_telemetry_missing:{command}"));
            continue;
        };
        let Some(object) = row.as_object() else {
            out.push(format!(
                "observability_command_telemetry_row_not_object:{command}"
            ));
            continue;
        };
        require_command_inventory_row(root, command, object, out);
    }
}

fn require_command_inventory_row(
    root: &Path,
    command: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    match row.get("observability_status").and_then(Value::as_str) {
        Some("observable") => require_observable_evidence(root, command, row, out),
        Some(status @ ("partially_observable" | "unobservable")) => {
            require_unobservable_metadata(command, row, out);
            out.push(format!(
                "observability_command_telemetry_unobservable:{command}:{status}"
            ));
        }
        Some(other) => out.push(format!(
            "observability_command_observability_status_invalid:{command}:{other}"
        )),
        None => out.push(format!(
            "observability_command_observability_status_missing:{command}"
        )),
    }
}

fn require_unobservable_metadata(command: &str, row: &Map<String, Value>, out: &mut Vec<String>) {
    if !owner_tracking(row)
        || !typed_row_accounting(row)
        || !super::row_contract::complete(row)
        || !non_empty_array(row, "missing_surfaces")
    {
        out.push(format!(
            "observability_command_telemetry_missing_metadata:{command}"
        ));
    }
}

fn typed_row_accounting(row: &Map<String, Value>) -> bool {
    row.get("observed_surfaces").is_some_and(Value::is_array)
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

fn require_observable_evidence(
    root: &Path,
    command: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    let required = [
        non_empty_array(row, "observed_surfaces"),
        empty_array(row, "missing_surfaces"),
        non_empty_string(row, "validator_check_id"),
        non_empty_array(row, "focused_tests"),
        non_empty_array(row, "receipt_paths"),
        non_empty_array(row, "live_query_proof_paths"),
        non_empty_array(row, "same_candidate_query_proof_paths"),
        owner_tracking(row),
        super::row_contract::observable(row),
        non_empty_string(row, "claim_impact"),
    ];
    if required.into_iter().any(|ok| !ok) {
        out.push(format!(
            "observability_command_telemetry_row_shape_only:{command}"
        ));
        return;
    }
    super::proof::require_current_receipts(root, command, row, out);
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
    non_empty_string(row, "current_owner_surface")
        && non_empty_string(row, "next_unobservable_surface")
}
