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
    let Some((run_id, correlation_id)) =
        receipt_run(root, "command", command, row, &candidate, &operation, out)
    else {
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
    let Some((run_id, correlation_id)) =
        receipt_run(root, "surface", surface, row, &candidate, operation, out)
    else {
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
    for rel in strings(row, "live_query_proof_paths") {
        let Ok(value) = crate::json_boundary::read_json(&root.join(&rel)) else {
            out.push(format!(
                "observability_{prefix}_fitting_query_missing:{command}:{rel}"
            ));
            continue;
        };
        if !query_current(&value, candidate, run_id) {
            out.push(format!(
                "observability_{prefix}_fitting_query_not_current:{command}:{rel}"
            ));
            continue;
        }
        if let Some(kind) = value.get("query_kind").and_then(Value::as_str) {
            kinds.insert(kind.to_string());
        }
        let rows_text = value.get("rows").map(Value::to_string).unwrap_or_default();
        if !rows_text.contains(correlation_id)
            || !rows_text.contains(candidate)
            || !rows_text.contains(operation)
        {
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

fn query_current(value: &Value, candidate: &str, run_id: &str) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::QUERY_SCHEMA)
        && value.get("status").and_then(Value::as_str) == Some("pass")
        && value.get("candidate_digest").and_then(Value::as_str) == Some(candidate)
        && value.get("run_id").and_then(Value::as_str) == Some(run_id)
}

fn receipt_run(
    root: &Path,
    prefix: &str,
    command: &str,
    row: &Map<String, Value>,
    candidate: &str,
    operation: &str,
    out: &mut Vec<String>,
) -> Option<(String, String)> {
    let rel = strings(row, "receipt_paths").into_iter().next()?;
    let Ok(value) = crate::json_boundary::read_json(&root.join(&rel)) else {
        out.push(format!(
            "observability_{prefix}_fitting_receipt_missing:{command}:{rel}"
        ));
        return None;
    };
    if !receipt_current(&value, candidate, operation) {
        out.push(format!(
            "observability_{prefix}_fitting_receipt_not_current:{command}:{rel}"
        ));
        return None;
    }
    Some((
        required_text(&value, "run_id", prefix, command, &rel, out)?,
        required_text(&value, "correlation_id", prefix, command, &rel, out)?,
    ))
}

fn receipt_current(value: &Value, candidate: &str, operation: &str) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(crate::cli::observe::types::RECEIPT_SCHEMA)
        && value.get("candidate_digest").and_then(Value::as_str) == Some(candidate)
        && value.get("operation").and_then(Value::as_str) == Some(operation)
        && match value.get("status").and_then(Value::as_str) {
            Some("pass") => true,
            Some("fail") => fail_closed_observability_receipt(value),
            _ => false,
        }
}

fn fail_closed_observability_receipt(value: &Value) -> bool {
    value.get("claim_ceiling").and_then(Value::as_str) == Some("withheld_or_blocked")
        && value
            .get("claim_impact")
            .and_then(Value::as_str)
            .is_some_and(|impact| impact.contains("block") || impact.contains("withheld"))
        && value
            .get("supported_claims")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && required_blocked_claims()
            .into_iter()
            .all(|claim| array_contains(value, "blocked_claims", claim))
}

fn required_blocked_claims() -> [&'static str; 6] {
    [
        "completion",
        "readiness",
        "release",
        "final_packet_correctness",
        "reviewer_exposure",
        "update_goal_eligibility",
    ]
}

fn array_contains(value: &Value, key: &str, expected: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn required_text(
    value: &Value,
    field: &str,
    prefix: &str,
    command: &str,
    rel: &str,
    out: &mut Vec<String>,
) -> Option<String> {
    let Some(text) = value.get(field).and_then(Value::as_str) else {
        out.push(format!(
            "observability_{prefix}_fitting_receipt_missing_{field}:{command}:{rel}"
        ));
        return None;
    };
    Some(text.to_string())
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
        _ => command.replace(' ', "."),
    }
}
