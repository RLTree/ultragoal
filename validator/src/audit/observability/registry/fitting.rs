use serde_json::{Map, Value};

pub(super) const REQUIRED_COMMANDS: &[&str] = &[
    "package digest",
    "source audit",
    "red fixture report",
    "schema validation",
    "mandatory-law validation",
    "standards-gardener rebind",
    "source-obligations check",
    "foundational-trace check",
    "coverage prove",
    "line-cap check",
    "namespace check",
    "product prove-fitness",
    "product prove-cohesion",
    "product prove-journey",
    "fit-repo prove",
    "review-round verify",
    "review-target build",
    "archive build",
    "final-packet prove",
    "registry probe",
    "install audit",
    "cache audit",
    "transaction finalize",
    "self law prove",
    "self update-goal eligibility",
    "rust toolchain verify",
    "rust fast",
    "rust standard",
    "rust release",
    "rust clean-proof",
    "rust watch",
    "rust memory prove",
    "rust dependency audit",
    "rust coverage prove",
    "rust workspace topology",
    "gc plan",
    "gc dry-run",
    "gc apply",
    "gc verify",
    "session-log hardening",
    "target-repo audit",
    "observe stack up",
    "observe stack health",
    "observe stack smoke",
    "observe stack down",
    "observe stack gc plan",
    "observe stack gc dry-run",
    "observe stack gc apply",
    "observe logs query",
    "observe metrics query",
    "observe traces query",
    "observe snapshot",
    "observe prove",
    "observe explain-failure",
    "observe explain-claim",
    "observe explain-check",
    "observe explain-law",
];

pub(super) fn check(value: &Value, out: &mut Vec<String>) {
    for command in REQUIRED_COMMANDS {
        if !commands_contain(value, command) {
            out.push(format!("observability_command_inventory_missing:{command}"));
        }
    }
    reject_unknown_inventory_commands(value, out);
    for key in row_requirement_keys() {
        if value.pointer(&format!("/row_requirements/{key}")) != Some(&Value::Bool(true)) {
            out.push(format!(
                "observability_command_inventory_requirement_missing:{key}"
            ));
        }
    }
    require_fitting_inventory(value, out);
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
    if let Some(rows) = value.get("fitting_inventory").and_then(Value::as_object) {
        for command in rows.keys() {
            if !REQUIRED_COMMANDS.contains(&command.as_str()) {
                out.push(format!("observability_command_fitting_unknown:{command}"));
            }
        }
    }
}

fn require_fitting_inventory(value: &Value, out: &mut Vec<String>) {
    let Some(rows) = value.get("fitting_inventory").and_then(Value::as_object) else {
        out.push("observability_command_fitting_inventory_missing".to_string());
        return;
    };
    for command in REQUIRED_COMMANDS {
        let Some(row) = rows.get(*command) else {
            out.push(format!("observability_command_fitting_missing:{command}"));
            continue;
        };
        let Some(object) = row.as_object() else {
            out.push(format!(
                "observability_command_fitting_row_not_object:{command}"
            ));
            continue;
        };
        require_fitting_row(command, object, out);
    }
}

fn require_fitting_row(command: &str, row: &Map<String, Value>, out: &mut Vec<String>) {
    match row.get("fitting_status").and_then(Value::as_str) {
        Some("fitted") => require_fitted_evidence(command, row, out),
        Some(status @ ("partially_fitted" | "unfitted")) => {
            require_unfitted_metadata(command, row, out);
            out.push(format!(
                "observability_command_fitting_unfitted:{command}:{status}"
            ));
        }
        Some(other) => out.push(format!(
            "observability_command_fitting_status_invalid:{command}:{other}"
        )),
        None => out.push(format!(
            "observability_command_fitting_status_missing:{command}"
        )),
    }
}

fn require_unfitted_metadata(command: &str, row: &Map<String, Value>, out: &mut Vec<String>) {
    if !non_empty_array(row, "missing_surfaces") || !non_empty_string(row, "claim_impact") {
        out.push(format!(
            "observability_command_fitting_missing_metadata:{command}"
        ));
    }
}

fn require_fitted_evidence(command: &str, row: &Map<String, Value>, out: &mut Vec<String>) {
    let required = [
        non_empty_array(row, "fitted_surfaces"),
        empty_array(row, "missing_surfaces"),
        non_empty_string(row, "validator_check_id"),
        non_empty_array(row, "focused_tests"),
        non_empty_array(row, "receipt_paths"),
        non_empty_array(row, "live_query_proof_paths"),
        non_empty_string(row, "claim_impact"),
    ];
    if required.into_iter().any(|ok| !ok) {
        out.push(format!(
            "observability_command_fitting_row_shape_only:{command}"
        ));
    }
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

fn row_requirement_keys() -> [&'static str; 10] {
    [
        "log_instrumentation",
        "metric_instrumentation",
        "trace_instrumentation",
        "pass_output_contract",
        "fail_output_contract",
        "receipt_observability_binding",
        "focused_tests",
        "claim_impact_mapping",
        "same_candidate_query_proof",
        "validator_enforced",
    ]
}
