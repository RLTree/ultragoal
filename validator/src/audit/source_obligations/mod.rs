use crate::json_boundary;
use serde_json::Value;
use std::path::Path;

mod required;

const WEAK_DISPOSITION_TERMS: &[&str] = &[
    "manual audit",
    "human audit",
    "deterministic_with_human_audit",
    "partial",
    "backlog",
    "backlogged",
    "blocked",
    "reviewer_and_backlog",
    "reviewer-only",
    "future",
    "missing receipt",
    "future receipt",
    "future validator",
    "prose-only",
    "row-shape-only",
    "stale-source-backed",
    "not_refreshed",
];

pub fn failures(root: &Path) -> Vec<String> {
    match json_boundary::read_json(&root.join("docs/source-obligation-matrix.json")) {
        Ok(value) => {
            let mut failures = value_failures(&value);
            failures.extend(super::foundational_law_trace::failures(root, &value));
            failures.extend(super::law::family::aliases::failures(root));
            failures
        }
        Err(err) => vec![format!("docs/source-obligation-matrix.json: {err}")],
    }
}

pub fn value_failures(value: &Value) -> Vec<String> {
    let Some(rows) = value.get("obligations").and_then(Value::as_array) else {
        return vec!["source_obligation_matrix_missing_rows".to_string()];
    };
    let mut failures = required_obligation_failures(rows);
    failures.extend(rows.iter().filter_map(row_failure));
    failures
}

pub(crate) fn required_obligation_failures(rows: &[Value]) -> Vec<String> {
    required::IDS
        .iter()
        .filter(|id| {
            !rows
                .iter()
                .any(|row| row.get("id").and_then(Value::as_str) == Some(**id))
        })
        .map(|id| {
            if *id == "namespace-progressive-disclosure" {
                "namespace_law_present_only_as_prose".to_string()
            } else {
                format!("{id}: source_obligation_missing_row")
            }
        })
        .collect()
}

pub(crate) fn row_failure(row: &Value) -> Option<String> {
    let id = row.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let disposition = row
        .get("enforcement_disposition")
        .and_then(Value::as_str)
        .unwrap_or("");
    if disposition.is_empty() {
        return Some(format!("{id}: source_obligation_missing_disposition"));
    }
    if let Some(term) = weak_row_term(row) {
        return Some(format!("{id}: source_obligation_weak_disposition:{term}"));
    }
    if row
        .get("missing_validation_fixture_or_receipt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        return Some(format!("{id}: source_obligation_missing_gap_field"));
    }
    let tokens: &[&str] = match id {
        "derived-authority-recomputation" => &["deterministic", "canonical", "recomput", "digest"],
        "authority-source-binding" => &["authority", "fallback", "receipt", "claim"],
        "source-installed-cache-alignment" => &["source", "installed", "cache", "receipt"],
        "namespace-progressive-disclosure" => &["namespace", "progressive", "validator", "red"],
        "validator-source-namespace-topology" => {
            &["validator", "source", "topology", "typed", "red", "tamper"]
        }
        _ => return None,
    };
    required_token_row_failure(row, id, tokens)
}

fn weak_row_term(row: &Value) -> Option<&'static str> {
    let merged = [
        "enforcement_disposition",
        "coverage_status",
        "missing_validation_fixture_or_receipt",
        "claim_ceiling_impact",
    ]
    .iter()
    .filter_map(|key| row.get(*key).and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    WEAK_DISPOSITION_TERMS
        .iter()
        .copied()
        .find(|term| merged.contains(term))
}

fn required_token_row_failure(row: &Value, id: &str, tokens: &[&str]) -> Option<String> {
    let merged = [
        "obligation",
        "package_surface",
        "enforcement_disposition",
        "coverage_status",
        "missing_validation_fixture_or_receipt",
        "claim_ceiling_impact",
    ]
    .iter()
    .filter_map(|key| row.get(*key).and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    for token in tokens {
        if !merged.contains(token) {
            return Some(format!("{id}: source_obligation_missing_{token}"));
        }
    }
    None
}
