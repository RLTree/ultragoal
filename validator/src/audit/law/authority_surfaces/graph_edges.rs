use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const CHECK_ID: &str = "authority-source-binding";

pub(super) fn require_mandatory_law_paths(
    root: &Path,
    inventory: &BTreeSet<String>,
    law: &str,
    row: &Value,
    red_ids: &BTreeSet<String>,
    out: &mut Vec<(String, String)>,
) {
    if let Some(rel) = text(row, "valid_fixture_path") {
        super::paths::require_path(root, inventory, law, "valid_fixture", rel, true, out);
    } else {
        push(
            out,
            format!("foundational_law_surface_missing_field:law={law};field=valid_fixture_path"),
        );
    }
    require_red_fixtures(root, inventory, law, row, red_ids, out);
    require_evidence_paths(root, inventory, law, row, out);
}

pub(super) fn require_trace_paths(
    root: &Path,
    inventory: &BTreeSet<String>,
    trace: &Value,
    out: &mut Vec<(String, String)>,
) {
    for row in trace_entries(trace) {
        let law = text(row, "law_id").unwrap_or("<missing-law-id>");
        if let Some(rel) = row
            .get("source_artifact")
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str)
        {
            super::paths::require_path(
                root,
                inventory,
                law,
                "foundational_source_artifact",
                rel,
                true,
                out,
            );
        }
        if let Some(rel) = text(row, "valid_fixture_id") {
            super::paths::require_path(
                root,
                inventory,
                law,
                "foundational_valid_fixture",
                rel,
                true,
                out,
            );
        }
        require_foundational_red_fixture(root, inventory, law, row, out);
        require_text(row, law, "validator_check_id", out);
        require_text(row, law, "claim_ceiling_impact", out);
    }
}

fn require_red_fixtures(
    root: &Path,
    inventory: &BTreeSet<String>,
    law: &str,
    row: &Value,
    red_ids: &BTreeSet<String>,
    out: &mut Vec<(String, String)>,
) {
    let Some(ids) = row.get("red_fixture_ids").and_then(Value::as_array) else {
        push(
            out,
            format!("foundational_law_surface_missing_field:law={law};field=red_fixture_ids"),
        );
        return;
    };
    for id in ids.iter().filter_map(Value::as_str) {
        let rel = format!("fixtures/red/{id}.json");
        if !red_ids.contains(id) {
            push(
                out,
                format!(
                    "foundational_law_surface_red_fixture_missing_from_catalog:law={law};red_fixture_id={id}"
                ),
            );
        }
        super::paths::require_path(root, inventory, law, "red_fixture", &rel, true, out);
    }
}

fn require_evidence_paths(
    root: &Path,
    inventory: &BTreeSet<String>,
    law: &str,
    row: &Value,
    out: &mut Vec<(String, String)>,
) {
    for rel in row
        .get("evidence_artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|artifact| artifact.get("path").and_then(Value::as_str))
    {
        super::paths::require_path(
            root,
            inventory,
            law,
            "evidence_artifact",
            rel,
            !super::paths::runtime_artifact(rel),
            out,
        );
    }
}

fn require_foundational_red_fixture(
    root: &Path,
    inventory: &BTreeSet<String>,
    law: &str,
    row: &Value,
    out: &mut Vec<(String, String)>,
) {
    if let Some(id) = text(row, "red_fixture_id") {
        let rel = format!("fixtures/red/{id}.json");
        super::paths::require_path(
            root,
            inventory,
            law,
            "foundational_red_fixture",
            &rel,
            true,
            out,
        );
    }
}

fn trace_entries(trace: &Value) -> impl Iterator<Item = &Value> {
    trace
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn require_text(row: &Value, law: &str, field: &str, out: &mut Vec<(String, String)>) {
    if text(row, field).is_none() {
        push(
            out,
            format!("foundational_law_surface_missing_field:law={law};field={field}"),
        );
    }
}

fn text<'a>(row: &'a Value, field: &str) -> Option<&'a str> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn push(out: &mut Vec<(String, String)>, detail: String) {
    out.push((CHECK_ID.to_string(), detail));
}
