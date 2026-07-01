use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn require_current_receipts(
    root: &Path,
    command: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    let Ok(candidate) = crate::package::inventory::package_digest(root) else {
        out.push(format!(
            "observability_command_fitting_candidate_digest_unavailable:{command}"
        ));
        return;
    };
    let operation = expected_operation(command);
    let Some((run_id, correlation_id)) = super::proof_receipt::receipt_run(
        root, "command", command, row, &candidate, &operation, out,
    ) else {
        return;
    };
    require_query_receipts(
        root,
        "command",
        command,
        row,
        &candidate,
        &run_id,
        &correlation_id,
        &operation,
        out,
    );
}

pub(super) fn require_current_surface_receipts(
    root: &Path,
    surface: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    let Ok(candidate) = crate::package::inventory::package_digest(root) else {
        out.push(format!(
            "observability_surface_fitting_candidate_digest_unavailable:{surface}"
        ));
        return;
    };
    let Some(operation) = row.get("operation").and_then(Value::as_str) else {
        out.push(format!(
            "observability_surface_fitting_operation_missing:{surface}"
        ));
        return;
    };
    let Some((run_id, correlation_id)) = super::proof_receipt::receipt_run(
        root, "surface", surface, row, &candidate, operation, out,
    ) else {
        return;
    };
    require_query_receipts(
        root,
        "surface",
        surface,
        row,
        &candidate,
        &run_id,
        &correlation_id,
        operation,
        out,
    );
}

fn require_query_receipts(
    root: &Path,
    prefix: &str,
    command: &str,
    row: &Map<String, Value>,
    candidate: &str,
    run_id: &str,
    correlation_id: &str,
    operation: &str,
    out: &mut Vec<String>,
) {
    let mut kinds = BTreeSet::new();
    for rel in strings(row, "same_candidate_query_proof_paths") {
        let Ok(value) = crate::json_boundary::read_json(&root.join(&rel)) else {
            out.push(format!(
                "observability_{prefix}_fitting_query_missing:{command}:{rel}"
            ));
            continue;
        };
        if explain_current(&value, candidate, run_id) {
            continue;
        }
        if !query_current(&value, candidate, run_id) {
            out.push(format!(
                "observability_{prefix}_fitting_query_not_current:{command}:{rel}"
            ));
            continue;
        }
        if let Some(kind) = value.get("query_kind").and_then(Value::as_str) {
            kinds.insert(kind.to_string());
        }
        let query_kind = value.get("query_kind").and_then(Value::as_str);
        let rows = value.get("rows");
        if !query_rows_match(query_kind, rows, candidate, correlation_id, operation) {
            out.push(format!(
                "observability_{prefix}_fitting_query_not_same_run:{command}:{rel}"
            ));
        }
    }
    for kind in ["logs", "metrics", "traces"] {
        if !kinds.contains(kind) {
            out.push(format!(
                "observability_{prefix}_fitting_query_kind_missing:{command}:{kind}"
            ));
        }
    }
}

fn explain_current(value: &Value, candidate: &str, run_id: &str) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::RECEIPT_SCHEMA)
        && value.get("status").and_then(Value::as_str) == Some("pass")
        && value.get("candidate_digest").and_then(Value::as_str) == Some(candidate)
        && value.get("operation").and_then(Value::as_str) == Some("observe.explain-failure")
        && value.get("run_id").and_then(Value::as_str) == Some(run_id)
}

fn query_rows_match(
    query_kind: Option<&str>,
    rows: Option<&Value>,
    candidate: &str,
    correlation_id: &str,
    operation: &str,
) -> bool {
    if query_kind == Some("metrics") {
        rows.is_some_and(|rows| super::metric::rows_match(rows, operation))
    } else {
        let rows_text = rows.map(Value::to_string).unwrap_or_default();
        rows_text.contains(correlation_id)
            && rows_text.contains(candidate)
            && rows_text.contains(operation)
    }
}

fn query_current(value: &Value, candidate: &str, run_id: &str) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::QUERY_SCHEMA)
        && value.get("status").and_then(Value::as_str) == Some("pass")
        && value.get("candidate_digest").and_then(Value::as_str) == Some(candidate)
        && value.get("run_id").and_then(Value::as_str) == Some(run_id)
}

fn strings(row: &Map<String, Value>, key: &str) -> Vec<String> {
    row.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn expected_operation(command: &str) -> String {
    match command {
        "red fixture report" => "red_fixture.report".to_string(),
        "update-goal eligibility" => "update_goal_eligibility".to_string(),
        "self update-goal eligibility" => "self_update_goal_eligibility".to_string(),
        _ => command.replace(' ', "."),
    }
}
