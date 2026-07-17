use serde_json::{Map, Value};
use std::path::Path;

pub(super) fn receipt_run(
    root: &Path,
    prefix: &str,
    command: &str,
    row: &Map<String, Value>,
    candidate: &str,
    operation: &str,
    out: &mut Vec<String>,
) -> Option<(String, String)> {
    let rel = strings(row, "receipt_paths").into_iter().next()?;
    let Ok(raw) = crate::json_boundary::read_json(&root.join(&rel)) else {
        out.push(format!(
            "observability_{prefix}_telemetry_receipt_missing:{command}:{rel}"
        ));
        return None;
    };
    let Some(value) = observability_binding(&raw) else {
        out.push(format!(
            "observability_{prefix}_telemetry_receipt_not_current:{command}:{rel}"
        ));
        return None;
    };
    if !receipt_current(value, candidate, operation) {
        out.push(format!(
            "observability_{prefix}_telemetry_receipt_not_current:{command}:{rel}"
        ));
        return None;
    }
    Some((
        required_text(value, "run_id", prefix, command, &rel, out)?,
        required_text(value, "correlation_id", prefix, command, &rel, out)?,
    ))
}

fn observability_binding(value: &Value) -> Option<&Value> {
    if value.get("schema").and_then(Value::as_str)
        == Some(crate::audit::observability::RECEIPT_SCHEMA)
    {
        Some(value)
    } else {
        value.get("observability").filter(|nested| {
            nested.get("schema").and_then(Value::as_str)
                == Some(crate::audit::observability::RECEIPT_SCHEMA)
        })
    }
}

fn receipt_current(value: &Value, candidate: &str, operation: &str) -> bool {
    value.get("candidate_digest").and_then(Value::as_str) == Some(candidate)
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
            .all(|claim| blocked_claim_present(value, claim))
}

fn blocked_claim_present(value: &Value, claim: &str) -> bool {
    let impact = value
        .get("claim_impact")
        .and_then(Value::as_str)
        .unwrap_or("");
    match claim {
        "readiness" => {
            array_contains(value, "blocked_claims", "readiness")
                || array_contains(value, "blocked_claims", "package_readiness")
                || array_contains(value, "blocked_claims", "review_readiness")
        }
        "release" => {
            array_contains(value, "blocked_claims", "release")
                || array_contains(value, "blocked_claims", "release_readiness")
        }
        "reviewer_exposure" => {
            array_contains(value, "blocked_claims", "reviewer_exposure")
                || array_contains(value, "blocked_claims", "app_registry_or_reviewer_exposure")
        }
        "final_packet_correctness" => {
            array_contains(value, "blocked_claims", claim) || impact.contains(claim)
        }
        _ => array_contains(value, "blocked_claims", claim),
    }
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
            "observability_{prefix}_telemetry_receipt_missing_{field}:{command}:{rel}"
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
