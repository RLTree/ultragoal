use crate::{digest, json_boundary};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const TRACE_PATH: &str = "docs/foundational-law-traceability.json";

pub fn failures(root: &Path, matrix: &Value) -> Vec<String> {
    let trace = match json_boundary::read_json(&root.join(TRACE_PATH)) {
        Ok(value) => value,
        Err(err) => return vec![format!("foundational_law_trace_load_failed:{err}")],
    };
    value_failures(root, matrix, &trace)
}

pub fn value_failures(root: &Path, matrix: &Value, trace: &Value) -> Vec<String> {
    let entries = match trace_entries(trace) {
        Ok(entries) => entries,
        Err(failure) => return vec![failure],
    };
    let context = trace_context(root, matrix);
    let mut out = Vec::new();
    out.extend(missing_required_failures(&entries, &context));
    out.extend(entry_id_failures(&entries, &context));
    for row in &entries {
        out.extend(row_failures(root, row, &context));
    }
    out
}

#[derive(Clone)]
pub(crate) struct TraceContext {
    required: BTreeSet<String>,
    standards: BTreeSet<String>,
    checks: BTreeSet<&'static str>,
    red: BTreeSet<String>,
}

pub(crate) fn trace_entries(trace: &Value) -> Result<Vec<Value>, String> {
    trace
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "foundational_law_trace_entries_missing".to_string())
}

pub(crate) fn trace_context(root: &Path, matrix: &Value) -> TraceContext {
    TraceContext {
        required: matrix_obligation_ids(matrix),
        standards: standards_row_ids(root),
        checks: crate::audit::contract::CHECK_IDS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        red: red_fixture_ids(root),
    }
}

pub(crate) fn missing_required_failures(entries: &[Value], context: &TraceContext) -> Vec<String> {
    let mut out = Vec::new();
    for id in &context.required {
        if !entries
            .iter()
            .any(|row| str_field(row, "obligation_id") == *id)
        {
            out.push(format!("foundational_law_trace_missing_obligation:{id}"));
        }
    }
    out
}

pub(crate) fn entry_id_failures(entries: &[Value], context: &TraceContext) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for row in entries {
        let id = str_field(row, "obligation_id");
        if id.is_empty() {
            out.push("foundational_law_trace_missing_obligation_id".to_string());
        } else if !context.required.contains(&id) {
            out.push(format!("foundational_law_trace_unknown_obligation:{id}"));
        }
        if !seen.insert(id.clone()) {
            out.push(format!("foundational_law_trace_duplicate_obligation:{id}"));
        }
    }
    out
}

pub(crate) fn row_failures(root: &Path, row: &Value, context: &TraceContext) -> Vec<String> {
    let id = str_field(row, "obligation_id");
    let mut out = Vec::new();
    if !source_artifact_valid(root, row) {
        out.push(format!("foundational_law_trace_source_stale:{id}"));
    }
    for key in ["law_id", "receipt_requirement", "claim_ceiling_impact"] {
        if str_field(row, key).is_empty() {
            out.push(format!("foundational_law_trace_missing_{key}:{id}"));
        }
    }
    let standard = str_field(row, "standards_row_id");
    if !context.standards.contains(&standard) {
        out.push(format!(
            "foundational_law_trace_unknown_standard:{id}:{standard}"
        ));
    }
    let check = str_field(row, "validator_check_id");
    if !context.checks.contains(check.as_str()) {
        out.push(format!("foundational_law_trace_unknown_check:{id}:{check}"));
    }
    let red_id = str_field(row, "red_fixture_id");
    if !context.red.contains(&red_id) {
        out.push(format!(
            "foundational_law_trace_unknown_red_fixture:{id}:{red_id}"
        ));
    }
    let valid = str_field(row, "valid_fixture_id");
    if valid.is_empty() || crate::package::inventory::resolve(root, &valid).is_err() {
        out.push(format!(
            "foundational_law_trace_valid_fixture_missing:{id}:{valid}"
        ));
    }
    out
}

fn source_artifact_valid(root: &Path, row: &Value) -> bool {
    let Some(source) = row.get("source_artifact").and_then(Value::as_object) else {
        return false;
    };
    let path = source.get("path").and_then(Value::as_str).unwrap_or("");
    let got = source.get("digest").and_then(Value::as_str).unwrap_or("");
    if path.is_empty() || got == crate::digest::ZERO || !got.starts_with("sha256:") {
        return false;
    }
    crate::package::inventory::resolve(root, path)
        .ok()
        .and_then(|path| digest::file(&path).ok())
        .is_some_and(|actual| actual == got)
}

fn matrix_obligation_ids(matrix: &Value) -> BTreeSet<String> {
    matrix
        .get("obligations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect()
}

fn standards_row_ids(root: &Path) -> BTreeSet<String> {
    json_boundary::read_json(&root.join("templates/agent-standards/enforcement.json"))
        .ok()
        .and_then(|value| value.get("rows").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect()
}

fn red_fixture_ids(root: &Path) -> BTreeSet<String> {
    json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect()
}

fn str_field(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
