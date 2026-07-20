use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn binding_failures(root: &Path, value: &Value, law: &str) -> Vec<String> {
    let mut out = Vec::new();
    let check_id = value
        .get("validator_check_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !crate::contract_check_ids::CHECK_IDS.contains(&check_id) {
        out.push(format!(
            "mandatory_law_unknown_validator_check:{law}:{check_id}"
        ));
        return out;
    }
    let valid = value
        .get("valid_fixture_path")
        .and_then(Value::as_str)
        .unwrap_or("");
    if valid != format!("fixtures/mandatory-law-surfaces/valid/{law}.json") {
        out.push(format!(
            "mandatory_law_green_fixture_not_law_bound:{law}:{valid}"
        ));
    }
    if !has_real_red_fixture(value) {
        out.push(format!("mandatory_law_missing_real_red_fixture:{law}"));
    }
    if let Some(fields) = value.get("law_specific").and_then(Value::as_object) {
        out.extend(specific_guard_red_fixture_failures(
            root, value, law, fields,
        ));
    }
    out
}

pub(super) fn current_check_failures(
    value: &Value,
    law: &str,
    failures: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let check_id = value
        .get("validator_check_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !crate::contract_check_ids::CHECK_IDS.contains(&check_id) {
        return Vec::new();
    }
    match failures.get(check_id) {
        Some(rows) if rows.is_empty() => Vec::new(),
        Some(_) => vec![format!(
            "mandatory_law_current_check_not_pass:{law}:{check_id}"
        )],
        None => vec![format!(
            "mandatory_law_current_check_missing:{law}:{check_id}"
        )],
    }
}

fn has_real_red_fixture(value: &Value) -> bool {
    value
        .get("red_fixture_ids")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter()
                .any(|row| row.as_str().is_some_and(|id| id.ends_with("-red")))
        })
}

fn specific_guard_red_fixture_failures(
    root: &Path,
    value: &Value,
    law: &str,
    fields: &serde_json::Map<String, Value>,
) -> Vec<String> {
    fields
        .iter()
        .filter(|(field, enabled)| {
            enabled.as_bool() == Some(true)
                && !specific_guard_has_red_fixture(root, value, law, field)
        })
        .map(|(field, _)| {
            format!("mandatory_law_specific_guard_missing_red_fixture:{law}:{field}")
        })
        .collect()
}

fn specific_guard_has_red_fixture(root: &Path, value: &Value, law: &str, field: &str) -> bool {
    value
        .get("red_fixture_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|id| red_fixture_enforces_guard(root, id, law, field))
}

fn red_fixture_enforces_guard(root: &Path, id: &str, law: &str, field: &str) -> bool {
    let path = root.join("fixtures/red").join(format!("{id}.json"));
    let Ok(value) = crate::json_boundary::read_json(&path) else {
        return false;
    };
    if value.get("id").and_then(Value::as_str) != Some(id) {
        return false;
    }
    let Some(expected) = value
        .get("expected_failure")
        .and_then(|failure| failure.get("error"))
        .and_then(Value::as_str)
    else {
        return false;
    };
    expected == format!("mandatory_law_specific_guard_not_enforced:{law}:{field}")
        || expected == field
}
