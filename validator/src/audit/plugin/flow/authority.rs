use serde_json::Value;
use std::collections::BTreeSet;

const REQUIRED_EDGES: &[&str] = &[
    "harness-ultragoal:fit-repo->validation_artifacts/harness/fit-repo-receipt.json",
    "harness-ultragoal:fit-repo->templates/.harness/coverage-manifest.json",
    "templates/scripts/check-coverage-full->coverage-proof-policy",
    "harness-ultragoal:product-fitness-gate->validation_artifacts/harness/product-fitness-receipt.json",
    "templates/scripts/check-agent-standards->agent-standards-enforcement",
];

pub(crate) fn failures(flow: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let declared = strings(flow, "validator_checks")
        .into_iter()
        .collect::<BTreeSet<_>>();
    for check in crate::audit::contract::CHECK_IDS {
        if !declared.contains(*check) {
            out.push(format!("plugin_flow_validator_check_missing:{check}"));
        }
    }
    let edge_set = flow
        .get("edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(edge_key)
        .collect::<BTreeSet<_>>();
    for edge in REQUIRED_EDGES {
        if !edge_set.contains(*edge) {
            out.push(format!("plugin_flow_required_edge_missing:{edge}"));
        }
    }
    out.extend(flow_receipt_failures(flow, &edge_set));
    out
}

fn flow_receipt_failures(flow: &Value, edge_set: &BTreeSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    let Some(rows) = flow.get("flows").and_then(Value::as_array) else {
        return vec!["plugin_flow_completion_receipt_missing:flows".to_string()];
    };
    for row in rows {
        let id = row.get("id").and_then(Value::as_str).unwrap_or("<unknown>");
        if row
            .get("completion_receipt")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            out.push(format!("plugin_flow_completion_receipt_missing:{id}"));
        }
        for edge in strings(row, "required_edges") {
            if !edge_set.contains(&edge) {
                out.push(format!("plugin_flow_required_edge_missing:{id}:{edge}"));
            }
        }
    }
    out
}

fn edge_key(value: &Value) -> Option<String> {
    Some(format!(
        "{}->{}",
        value.get("from")?.as_str()?,
        value.get("to")?.as_str()?
    ))
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}
