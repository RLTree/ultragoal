use crate::json_boundary;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod gate92;
#[cfg(test)]
mod tests;

pub(super) fn coverage_failures(
    requirements: &BTreeMap<String, String>,
    trace: &BTreeMap<String, Value>,
) -> Vec<String> {
    requirements
        .keys()
        .filter(|id| !trace.contains_key(*id))
        .map(|id| format!("research_trace_missing_requirement:{id}"))
        .collect()
}

pub(super) fn row_failures(
    root: &Path,
    requirements: &BTreeMap<String, String>,
    trace: &BTreeMap<String, Value>,
) -> Vec<String> {
    let standards = ids_json(
        root,
        "templates/agent-standards/enforcement.json",
        "rows",
        "id",
    );
    let obligations = ids_json(
        root,
        "docs/source-obligation-matrix.json",
        "obligations",
        "id",
    );
    let traces = ids_json(
        root,
        "docs/foundational-law-traceability.json",
        "entries",
        "obligation_id",
    );
    let red = ids_red(root);
    let package_paths = inventory_ids(root);
    let checks = crate::contract_check_ids::CHECK_IDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let known = KnownIds {
        standards: &standards,
        obligations: &obligations,
        traces: &traces,
        red: &red,
        checks: &checks,
        package_paths: &package_paths,
    };
    trace
        .iter()
        .flat_map(|(id, row)| row_failures_for(root, requirements, id, row, &known))
        .collect()
}

struct KnownIds<'a> {
    standards: &'a BTreeSet<String>,
    obligations: &'a BTreeSet<String>,
    traces: &'a BTreeSet<String>,
    red: &'a BTreeSet<String>,
    checks: &'a BTreeSet<&'static str>,
    package_paths: &'a BTreeSet<String>,
}

fn row_failures_for(
    root: &Path,
    requirements: &BTreeMap<String, String>,
    id: &str,
    row: &Value,
    known: &KnownIds<'_>,
) -> Vec<String> {
    let mut out = Vec::new();
    match requirements.get(id) {
        Some(source) if source == &text(row, "source_id") => {}
        Some(source) => out.push(format!("research_trace_source_mismatch:{id}:{source}")),
        None => out.push(format!("research_trace_unknown_requirement:{id}")),
    }
    out.extend(check_ids(
        id,
        row,
        "standards_row_ids",
        known.standards,
        "unknown_standard",
    ));
    out.extend(check_ids(
        id,
        row,
        "source_obligation_ids",
        known.obligations,
        "unknown_source_obligation",
    ));
    out.extend(check_ids(
        id,
        row,
        "foundational_trace_ids",
        known.traces,
        "unknown_foundational_trace",
    ));
    out.extend(check_checks(id, row, known.checks));
    out.extend(check_ids(
        id,
        row,
        "red_fixture_ids",
        known.red,
        "unknown_red_fixture",
    ));
    out.extend(check_ids(
        id,
        row,
        "tamper_fixture_ids",
        known.red,
        "unknown_tamper_fixture",
    ));
    out.extend(paths(
        root,
        id,
        row,
        "green_fixture_paths",
        "missing_green_fixture",
        None,
    ));
    out.extend(paths(root, id, row, "schemas", "missing_schema", None));
    out.extend(paths(
        root,
        id,
        row,
        "package_inventory_paths",
        "package_omitted",
        Some(known.package_paths),
    ));
    out.extend(paths(
        root,
        id,
        row,
        "setup_retrofit_outputs",
        "setup_retrofit_omitted",
        Some(known.package_paths),
    ));
    out.extend(gate92::required_field_failures(id, row));
    out.extend(gate92::row_failures(id, row));
    if array(row, "canonical_law_ids")
        .iter()
        .all(|law| law.starts_with("HU-"))
    {
        out.push(format!("research_trace_alias_only:{id}"));
    }
    out
}

fn check_checks(id: &str, row: &Value, known: &BTreeSet<&str>) -> Vec<String> {
    array(row, "validator_check_ids")
        .into_iter()
        .filter(|check| !known.contains(check.as_str()))
        .map(|check| format!("research_trace_unknown_check:{id}:{check}"))
        .collect()
}

fn check_ids(
    id: &str,
    row: &Value,
    field: &str,
    known: &BTreeSet<String>,
    code: &str,
) -> Vec<String> {
    array(row, field)
        .into_iter()
        .filter(|item| !known.contains(item))
        .map(|item| format!("research_trace_{code}:{id}:{item}"))
        .collect()
}

fn paths(
    root: &Path,
    id: &str,
    row: &Value,
    field: &str,
    code: &str,
    manifest_paths: Option<&BTreeSet<String>>,
) -> Vec<String> {
    array(row, field)
        .into_iter()
        .filter(|path| {
            let resolved = crate::package::inventory::resolve(root, path);
            let missing = resolved
                .as_ref()
                .map_or(true, |resolved| !resolved.exists());
            let omitted = manifest_paths.is_some_and(|known| !known.contains(path));
            missing || omitted
        })
        .map(|path| format!("research_trace_{code}:{id}:{path}"))
        .collect()
}

fn inventory_ids(root: &Path) -> BTreeSet<String> {
    json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .ok()
        .map(|manifest| crate::package::inventory::inventory_paths(&manifest))
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn ids_json(root: &Path, rel: &str, array_key: &str, id_key: &str) -> BTreeSet<String> {
    json_boundary::read_json(&root.join(rel))
        .ok()
        .and_then(|value| value.get(array_key).and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get(id_key).and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn ids_red(root: &Path) -> BTreeSet<String> {
    json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn array(row: &Value, key: &str) -> Vec<String> {
    row.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn text(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
