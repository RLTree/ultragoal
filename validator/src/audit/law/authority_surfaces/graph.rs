use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const CHECK_ID: &str = "authority-source-binding";

pub(crate) struct AuthorityGraphInputs<'a> {
    pub root: &'a Path,
    pub inventory: &'a BTreeSet<String>,
    pub mandatory: &'a Value,
    pub obligations: &'a Value,
    pub trace: &'a Value,
    pub standards: &'a Value,
    pub standards_audit: &'a str,
    pub red_ids: &'a BTreeSet<String>,
}

pub(super) fn failures(
    root: &Path,
    inventory: &BTreeSet<String>,
    red_ids: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let mandatory = super::paths::read_json(root, "docs/mandatory-law-surfaces.json");
    let obligations = super::paths::read_json(root, "docs/source-obligation-matrix.json");
    let trace = super::paths::read_json(root, "docs/foundational-law-traceability.json");
    let standards = super::paths::read_json(root, "templates/agent-standards/enforcement.json");
    let standards_audit =
        std::fs::read_to_string(root.join("templates/agent-standards/enforcement-audit.tsv"))
            .unwrap_or_default();
    authority_graph_failures(AuthorityGraphInputs {
        root,
        inventory,
        mandatory: &mandatory,
        obligations: &obligations,
        trace: &trace,
        standards: &standards,
        standards_audit: &standards_audit,
        red_ids,
    })
}

pub(crate) fn authority_graph_failures(inputs: AuthorityGraphInputs<'_>) -> Vec<(String, String)> {
    let obligation_ids = ids(inputs.obligations, "obligations", "id");
    let trace_law_ids = ids(inputs.trace, "entries", "law_id");
    let standards_ids = ids(inputs.standards, "rows", "id");
    let audit_ids = audit_row_ids(inputs.standards_audit);
    let mut out = Vec::new();
    for row in inputs.mandatory
        .get("laws")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let law = text(row, "law_id").unwrap_or("<missing-law-id>");
        require_text(row, law, "validator_check_id", &mut out);
        require_text(row, law, "claim_ceiling_guard", &mut out);
        require_row_binding(
            row,
            law,
            "source_obligation_id",
            &obligation_ids,
            "source_obligation",
            &mut out,
        );
        require_row_binding(
            row,
            law,
            "foundational_trace_id",
            &trace_law_ids,
            "foundational_trace",
            &mut out,
        );
        require_row_binding(
            row,
            law,
            "standards_row_id",
            &standards_ids,
            "standards",
            &mut out,
        );
        if let Some(standards_row) = text(row, "standards_row_id")
            && !audit_ids.contains(standards_row)
        {
            push(
                &mut out,
                format!(
                    "foundational_law_surface_missing_standards_audit_row:law={law};standards_row_id={standards_row}"
                ),
            );
        }
        super::graph_edges::require_mandatory_law_paths(
            inputs.root, inputs.inventory, law, row, inputs.red_ids, &mut out,
        );
    }
    super::graph_edges::require_trace_paths(inputs.root, inputs.inventory, inputs.trace, &mut out);
    out
}

fn require_text(row: &Value, law: &str, field: &str, out: &mut Vec<(String, String)>) {
    if text(row, field).is_none() {
        push(
            out,
            format!("foundational_law_surface_missing_field:law={law};field={field}"),
        );
    }
}

fn require_row_binding(
    row: &Value,
    law: &str,
    field: &str,
    ids: &BTreeSet<String>,
    surface: &str,
    out: &mut Vec<(String, String)>,
) {
    let Some(id) = text(row, field) else {
        push(
            out,
            format!("foundational_law_surface_missing_field:law={law};field={field}"),
        );
        return;
    };
    if !ids.contains(id) {
        push(
            out,
            format!(
                "foundational_law_surface_missing_{surface}_row:law={law};field={field};id={id}"
            ),
        );
    }
}

fn ids(value: &Value, array_key: &str, id_key: &str) -> BTreeSet<String> {
    value
        .get(array_key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get(id_key).and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

fn audit_row_ids(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| line.split('\t').next())
        .filter(|id| !id.is_empty() && *id != "id")
        .map(str::to_string)
        .collect()
}

fn text<'a>(row: &'a Value, field: &str) -> Option<&'a str> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn push(out: &mut Vec<(String, String)>, detail: String) {
    out.push((CHECK_ID.to_string(), detail));
}
