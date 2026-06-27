use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn binding_failures(value: &Value, law: &str) -> Vec<String> {
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
