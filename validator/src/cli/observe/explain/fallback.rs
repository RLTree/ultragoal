use serde_json::{Value, json};
use std::path::Path;

pub(super) fn failure_summary(value: &Value) -> Option<String> {
    let failures = value.as_array()?;
    let first = failures.first()?.as_str()?;
    if first == "no current source-audit failure; inspect final-packet/control receipts" {
        return None;
    }
    Some(
        failures
            .iter()
            .filter_map(Value::as_str)
            .take(8)
            .collect::<Vec<_>>()
            .join("; "),
    )
}

pub(super) fn current_failure(root: &Path) -> Value {
    let packet = root.join("validation_artifacts/review/final-packet-proof.json");
    if let Some(failures) = crate::json_boundary::read_json(&packet)
        .ok()
        .and_then(|value| value.pointer("/failure/observed_failures").cloned())
        .filter(|value| value.as_array().is_some_and(|items| !items.is_empty()))
    {
        return failures;
    }
    let path = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let Some(value) = crate::json_boundary::read_json(&path).ok() else {
        return json!(["source audit receipt unavailable"]);
    };
    if let Some(failures) = value
        .get("failures")
        .cloned()
        .filter(|value| value.as_array().is_some_and(|items| !items.is_empty()))
    {
        return failures;
    }
    let check_failures = check_failures(&value);
    if check_failures.is_empty() {
        json!(["no current source-audit failure; inspect final-packet/control receipts"])
    } else {
        Value::Array(check_failures)
    }
}

fn check_failures(value: &Value) -> Vec<Value> {
    if let Some(rows) = value.get("checks").and_then(Value::as_array) {
        return rows
            .iter()
            .filter(|check| check.get("status").and_then(Value::as_str) != Some("pass"))
            .filter_map(|check| {
                check
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|id| Value::String(id.to_string()))
            })
            .collect();
    }
    value
        .get("checks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|checks| checks.iter())
        .filter(|(_, check)| check.get("status").and_then(Value::as_str) != Some("pass"))
        .map(|(id, check)| {
            let details = check
                .get("details")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty() && *text != "pass");
            match details {
                Some(details) => Value::String(format!("{id}: {details}")),
                None => Value::String(id.clone()),
            }
        })
        .collect()
}
