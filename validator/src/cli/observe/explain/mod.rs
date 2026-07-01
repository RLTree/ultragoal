use crate::cli::observe::telemetry;
use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let audit = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let observed_run = command
        .run_id
        .as_deref()
        .and_then(|run_id| run_event(root, run_id));
    let known_current_failure = observed_run
        .as_ref()
        .and_then(event_failure)
        .unwrap_or_else(|| current_failure(root));
    let failure_summary = failure_summary(&known_current_failure);
    let mut receipt = telemetry::base_receipt(root, command, "pass", failure_summary.as_deref())?;
    let repair_guidance = repair_guidance(observed_run.as_ref());
    if let Some(event) = observed_run {
        promote_observed_failure_fields(&mut receipt, &event);
        receipt["observed_run"] = event;
    }
    receipt["explanation"] = json!({
        "requested_run_id": command.run_id,
        "requested_claim_id": command.claim_id,
        "requested_check_id": command.check_id,
        "requested_law_id": command.law_id,
        "current_source_audit_receipt": "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "current_source_audit_digest": crate::digest::file(&audit).unwrap_or_else(|_| crate::digest::ZERO.to_string()),
        "known_current_failure": known_current_failure,
        "repair_guidance": repair_guidance
    });
    Ok(receipt)
}

fn repair_guidance(observed_run: Option<&Value>) -> String {
    observed_run
        .and_then(|event| event.get("next_repair"))
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty() && *text != "none")
        .unwrap_or(
            "Inspect final-packet proof/source-audit digest dereference, regenerate same-candidate lower-level receipts, and rerun source audit once after implementation changes.",
        )
        .to_string()
}

fn promote_observed_failure_fields(receipt: &mut Value, event: &Value) {
    for (target, source) in [
        ("observed_failure_class", "failure_class"),
        ("observed_why_failed", "why_failed"),
        ("observed_where_failed", "where_failed"),
        ("observed_next_repair", "next_repair"),
        ("observed_claim_impact", "claim_impact"),
    ] {
        if event
            .get(source)
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty() && text != "none")
        {
            receipt[target] = event[source].clone();
        }
    }
    for key in [
        "law_id",
        "check_id",
        "claim_id",
        "failure_class",
        "where_failed",
        "next_repair",
        "claim_impact",
        "query_hint_logql",
        "query_hint_promql",
        "query_hint_traceql",
    ] {
        if event
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty() && text != "none")
        {
            receipt[key] = event[key].clone();
        }
    }
}

fn event_failure(event: &Value) -> Option<Value> {
    let status = event.get("status").and_then(Value::as_str)?;
    if status == "pass" {
        return None;
    }
    let why = event.get("why_failed").and_then(Value::as_str)?;
    if why.trim().is_empty() || why == "none" {
        return None;
    }
    Some(json!([why]))
}

fn run_event(root: &Path, run_id: &str) -> Option<Value> {
    spool_event(root, run_id).or_else(|| receipt_event(root, run_id))
}

fn spool_event(root: &Path, run_id: &str) -> Option<Value> {
    let path = root.join("validation_artifacts/observability/spool/events.jsonl");
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().rev().find_map(|line| {
        let value: Value = serde_json::from_str(line).ok()?;
        (value.get("run_id").and_then(Value::as_str) == Some(run_id)
            && event_failure(&value).is_some())
        .then_some(value)
    })
}

fn receipt_event(root: &Path, run_id: &str) -> Option<Value> {
    let dir = root.join("validation_artifacts/observability");
    let mut paths = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths.into_iter().rev().find_map(|path| {
        let value = crate::json_boundary::read_json(&path).ok()?;
        let event = value
            .get("event")
            .and_then(|event| {
                (event.get("run_id").and_then(Value::as_str) == Some(run_id)).then(|| event.clone())
            })
            .or_else(|| {
                (value.get("run_id").and_then(Value::as_str) == Some(run_id))
                    .then(|| fallback_event_from_receipt(&value))
            })?;
        event_failure(&event).is_some().then_some(event)
    })
}

fn fallback_event_from_receipt(value: &Value) -> Value {
    json!({
        "run_id": value.get("run_id").cloned().unwrap_or(Value::Null),
        "status": value.get("status").cloned().unwrap_or(Value::Null),
        "failure_class": value.get("failure_class").cloned().unwrap_or(Value::Null),
        "why_failed": value.get("why_failed").cloned().unwrap_or(Value::Null),
        "where_failed": value.get("where_failed").cloned().unwrap_or(Value::Null),
        "next_repair": value.get("next_repair").cloned().unwrap_or(Value::Null),
        "claim_impact": value.get("claim_impact").cloned().unwrap_or(Value::Null),
        "law_id": value.get("law_id").cloned().unwrap_or(Value::Null),
        "check_id": value.get("check_id").cloned().unwrap_or(Value::Null),
        "claim_id": value.get("claim_id").cloned().unwrap_or(Value::Null),
        "query_hint_logql": value.get("query_hint_logql").cloned().unwrap_or(Value::Null),
        "query_hint_promql": value.get("query_hint_promql").cloned().unwrap_or(Value::Null),
        "query_hint_traceql": value.get("query_hint_traceql").cloned().unwrap_or(Value::Null)
    })
}

fn failure_summary(value: &Value) -> Option<String> {
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

fn current_failure(root: &Path) -> Value {
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
    let check_failures: Vec<Value> = check_failures(&value);
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

#[cfg(test)]
mod tests;
