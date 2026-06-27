use crate::audit::contract::Failure;
use crate::claim_semantics::{object_by_id, str_field};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn check_backlog(backlog: &Value, claims: &BTreeMap<String, &Value>, out: &mut Vec<Failure>) {
    let rows = backlog
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    duplicate_claim_rows(&rows, out);
    let row_value = Value::Array(rows.clone());
    let rows_by_id = object_by_id(Some(&row_value));
    for claim in claims.values() {
        claim_row_join(claim, &rows_by_id, out);
    }
    for row in rows {
        row_checks(&row, claims, out);
    }
}

fn duplicate_claim_rows(rows: &[Value], out: &mut Vec<Failure>) {
    let claim_ids = rows
        .iter()
        .map(|row| str_field(row, "claim_id"))
        .collect::<Vec<_>>();
    if claim_ids.iter().collect::<BTreeSet<_>>().len() != rows.len() {
        out.push(Failure::new(
            "backlog-row-coverage",
            "backlog_row_claim_duplicate_or_missing",
            "claim_id",
        ));
    }
}

fn claim_row_join(claim: &Value, rows_by_id: &BTreeMap<String, &Value>, out: &mut Vec<Failure>) {
    let row_id = str_field(claim, "backlog_row_id");
    if let Some(row) = rows_by_id.get(&row_id)
        && str_field(row, "claim_id") != str_field(claim, "id")
    {
        out.push(Failure::new(
            "backlog-row-coverage",
            "backlog_row_claim_mismatch",
            row_id,
        ));
    }
}

fn row_checks(row: &Value, claims: &BTreeMap<String, &Value>, out: &mut Vec<Failure>) {
    if !claims.contains_key(&str_field(row, "claim_id")) {
        out.push(Failure::new(
            "backlog-row-coverage",
            "backlog_row_claim_missing",
            str_field(row, "id"),
        ));
    }
    if row
        .get("attempts")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push(Failure::new(
            "backlog-attempt-proof",
            "blocker_without_attempt_evidence",
            str_field(row, "id"),
        ));
    }
    for attempt in row
        .get("attempts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if str_field(attempt, "action").contains("Generic attempted") {
            out.push(Failure::new(
                "backlog-attempt-proof",
                "fake_generic_attempt_satisfies_blocker",
                str_field(attempt, "id"),
            ));
        }
    }
}
