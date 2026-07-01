use serde_json::{Map, Value};

pub(super) fn complete(row: &Map<String, Value>) -> bool {
    surface_contracts(row) && explicit_fields(row)
}

pub(super) fn fitted(row: &Map<String, Value>) -> bool {
    complete(row)
        && [
            "same_candidate_query_proof_paths",
            "red_fixtures",
            "green_fixtures",
            "tamper_fixtures",
        ]
        .into_iter()
        .all(|key| non_empty_array(row, key))
}

fn surface_contracts(row: &Map<String, Value>) -> bool {
    [
        &["log"][..],
        &["metric"][..],
        &["trace"][..],
        &["pass", "stdout"][..],
        &["fail", "stdout"][..],
        &["receipt", "observability"][..],
    ]
    .into_iter()
    .all(|terms| row_accounts_for(row, terms))
}

fn explicit_fields(row: &Map<String, Value>) -> bool {
    [
        "log_instrumentation",
        "metric_instrumentation",
        "trace_instrumentation",
        "pass_stdout_contract",
        "fail_stdout_contract",
        "receipt_observability_binding",
        "claim_impact",
    ]
    .into_iter()
    .all(|key| non_empty_string(row, key))
        && [
            "focused_tests",
            "receipt_paths",
            "live_query_proof_paths",
            "same_candidate_query_proof_paths",
            "red_fixtures",
            "green_fixtures",
            "tamper_fixtures",
        ]
        .into_iter()
        .all(|key| row.get(key).is_some_and(Value::is_array))
}

fn row_accounts_for(row: &Map<String, Value>, terms: &[&str]) -> bool {
    ["fitted_surfaces", "missing_surfaces"]
        .into_iter()
        .flat_map(|key| row.get(key).and_then(Value::as_array).into_iter().flatten())
        .filter_map(Value::as_str)
        .any(|surface| {
            let lower = surface.to_ascii_lowercase();
            terms.iter().all(|term| lower.contains(term))
        })
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
