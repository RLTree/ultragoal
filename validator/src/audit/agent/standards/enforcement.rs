use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const ENFORCEMENT_JSON: &str = "templates/agent-standards/enforcement.json";
const ENFORCEMENT_TSV: &str = "templates/agent-standards/enforcement.tsv";
const AUDIT_TSV: &str = "templates/agent-standards/enforcement-audit.tsv";
const CHECK_SCRIPT: &str = "templates/scripts/check-agent-standards";
const REQUIRED_IDS: &[&str] = crate::audit::agent::standards::ids::REQUIRED_IDS;

pub fn failures(root: &Path) -> Vec<String> {
    failures_for_paths(
        root,
        ENFORCEMENT_JSON,
        ENFORCEMENT_TSV,
        AUDIT_TSV,
        CHECK_SCRIPT,
    )
}

pub fn failures_for_paths(
    root: &Path,
    enforcement_json: &str,
    enforcement_tsv: &str,
    audit_tsv: &str,
    check_script: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    for path in [enforcement_json, enforcement_tsv, audit_tsv, check_script] {
        if !root.join(path).is_file() {
            out.push(format!("agent_standards_surface_missing:{path}"));
        }
    }
    let json_rows = match crate::json_boundary::read_json(&root.join(enforcement_json)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("agent_standards_json_load_failed:{err}"));
            return out;
        }
    };
    out.extend(value_failures(&json_rows, Some(root)));
    out.extend(crate::audit::agent::standards::tsv::checks::failures(
        root,
        enforcement_tsv,
        audit_tsv,
        &json_rows,
    ));
    out
}

pub fn value_failures(value: &Value, root: Option<&Path>) -> Vec<String> {
    let rows = match value.get("rows").and_then(Value::as_array) {
        Some(rows) => rows,
        None => return vec!["agent_standards_rows_missing".to_string()],
    };
    let mut out = required_id_failures(rows);
    let mut seen = BTreeSet::new();
    for row in rows {
        let id = str_field(row, "id");
        if !seen.insert(id.clone()) {
            out.push(format!("agent_standards_duplicate_row:{id}"));
        }
        out.extend(row_failures(row, root));
    }
    out
}

fn required_id_failures(rows: &[Value]) -> Vec<String> {
    REQUIRED_IDS
        .iter()
        .filter(|id| {
            !rows
                .iter()
                .any(|row| row.get("id").and_then(Value::as_str) == Some(**id))
        })
        .map(|id| format!("agent_standards_required_row_missing:{id}"))
        .collect()
}

fn row_failures(row: &Value, root: Option<&Path>) -> Vec<String> {
    let id = str_field(row, "id");
    let status = str_field(row, "enforcement_status");
    let mut out = Vec::new();
    if id.is_empty() {
        out.push("agent_standards_row_missing_id".to_string());
    }
    for key in [
        "source_law",
        "required_behavior",
        "owner_lane",
        "current_status",
    ] {
        if str_field(row, key).is_empty() {
            out.push(format!("agent_standards_row_missing_{key}:{id}"));
        }
    }
    if str_field(row, "blocker_or_repair_action").is_empty() {
        out.push(format!("agent_standards_debt_without_action:{id}"));
    }
    if row
        .get("required_execplan_refs")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push(format!("agent_standards_execplan_refs_missing:{id}"));
    }
    match status.as_str() {
        "mechanized" => mechanized_failures(row, root, &mut out),
        "backlogged" | "blocked" => out.push(format!(
            "agent_standards_unmechanized_material_claim:{id}:{status}"
        )),
        "informational" => {
            if str_field(row, "claim_ids_affected") != "none" {
                out.push(format!("agent_standards_informational_overclaims:{id}"));
            }
        }
        "" | "unclassified" | "unknown" => {
            out.push(format!("agent_standards_row_unclassified:{id}"));
        }
        _ => out.push(format!("agent_standards_status_invalid:{id}")),
    }
    out
}

fn mechanized_failures(row: &Value, root: Option<&Path>, out: &mut Vec<String>) {
    let id = str_field(row, "id");
    let path = str_field(row, "gate_or_fixture_path");
    if path.is_empty() {
        out.push(format!("agent_standards_mechanized_without_gate:{id}"));
        return;
    }
    if let Some(root) = root
        && (crate::package::inventory::package_path_error(root, &path).is_some()
            || !gate_path_exists(root, &path))
    {
        out.push(format!("agent_standards_gate_missing:{id}:{path}"));
    }
}

fn gate_path_exists(root: &Path, path: &str) -> bool {
    root.join(path).is_file()
        || path
            .strip_prefix("scripts/")
            .is_some_and(|tail| root.join("templates/scripts").join(tail).is_file())
}

fn str_field(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
