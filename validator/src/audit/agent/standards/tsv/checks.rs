use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const ROW_HEADER: &[&str] = &[
    "id",
    "source_law",
    "required_behavior",
    "enforcement_status",
    "gate_or_fixture_path",
    "owner_lane",
    "claim_ids_affected",
    "current_status",
    "blocker_or_repair_action",
    "required_execplan_refs",
];

const AUDIT_HEADER: &[&str] = &[
    "standard_id",
    "audit_status",
    "evidence_path",
    "evidence_digest",
    "audited_at",
    "claim_ceiling_impact",
];

pub fn failures(
    root: &Path,
    enforcement_tsv: &str,
    audit_tsv: &str,
    json_rows: &Value,
) -> Vec<String> {
    let rows = match crate::audit::agent::standards::tsv::parse(root, enforcement_tsv, ROW_HEADER) {
        Ok(rows) => rows,
        Err(err) => return vec![err],
    };
    let audits = match crate::audit::agent::standards::tsv::parse(root, audit_tsv, AUDIT_HEADER) {
        Ok(rows) => rows,
        Err(err) => return vec![err],
    };
    let mut out = row_drift_failures(&rows, json_rows);
    out.extend(audit_failures(root, &audits, json_rows));
    out
}

fn row_drift_failures(rows: &[BTreeMap<String, String>], json_rows: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let row_index = rows
        .iter()
        .map(|row| (field(row, "id"), field(row, "enforcement_status")))
        .collect::<BTreeMap<_, _>>();
    for row in json_rows_array(json_rows) {
        let id = str_field(row, "id");
        if row_index.get(&id) != Some(&str_field(row, "enforcement_status")) {
            out.push(format!("agent_standards_tsv_json_mismatch:{id}"));
        }
        if tsv_refs(rows, &id) != json_refs(row) {
            out.push(format!("agent_standards_execplan_refs_drift:{id}"));
        }
    }
    out
}

fn audit_failures(
    root: &Path,
    audits: &[BTreeMap<String, String>],
    json_rows: &Value,
) -> Vec<String> {
    let mut out = Vec::new();
    let audit_ids = audits
        .iter()
        .map(|row| field(row, "standard_id"))
        .collect::<BTreeSet<_>>();
    for row in json_rows_array(json_rows) {
        let id = str_field(row, "id");
        if str_field(row, "enforcement_status") != "informational" && !audit_ids.contains(&id) {
            out.push(format!("agent_standards_audit_missing:{id}"));
        }
    }
    for audit in audits {
        if field(audit, "audit_status") != "pass" {
            out.push(format!(
                "agent_standards_audit_not_pass:{}:{}",
                field(audit, "standard_id"),
                field(audit, "audit_status")
            ));
        }
        if field(audit, "audit_status") == "pass" && evidence_invalid(root, audit) {
            out.push(format!(
                "agent_standards_audit_evidence_invalid:{}",
                field(audit, "standard_id")
            ));
        }
    }
    out
}

fn evidence_invalid(root: &Path, audit: &BTreeMap<String, String>) -> bool {
    let got = field(audit, "evidence_digest");
    if got == crate::digest::ZERO || !got.starts_with("sha256:") {
        return true;
    }
    let Some(path) = evidence_path(root, &field(audit, "evidence_path")) else {
        return true;
    };
    crate::digest::file(&path).map_or(true, |actual| actual != got)
}

fn evidence_path(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() || crate::package::inventory::package_path_error(root, rel).is_some() {
        return None;
    }
    let direct = root.join(rel);
    if direct.is_file() {
        return Some(direct);
    }
    rel.strip_prefix("scripts/")
        .map(|tail| root.join("templates/scripts").join(tail))
        .filter(|path| path.is_file())
}

fn json_rows_array(json_rows: &Value) -> impl Iterator<Item = &Value> {
    json_rows
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn json_refs(row: &Value) -> String {
    row.get("required_execplan_refs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(";")
}

fn tsv_refs(rows: &[BTreeMap<String, String>], id: &str) -> String {
    rows.iter()
        .find(|tsv| field(tsv, "id") == id)
        .map(|tsv| field(tsv, "required_execplan_refs"))
        .unwrap_or_default()
}

fn field(row: &BTreeMap<String, String>, key: &str) -> String {
    crate::audit::agent::standards::tsv::field(row, key)
}

fn str_field(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
