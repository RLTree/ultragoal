use crate::audit::contract::Failure;
use crate::digest;
use crate::json_boundary;
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn product_receipt_evidence_checks(
    ev: &Value,
    evidence_paths: &BTreeSet<&str>,
    ui_paths: &BTreeSet<&str>,
    out: &mut Vec<Failure>,
) {
    let task = &ev["product_cohesion_task"];
    let mut text = format!(
        "{} {}",
        str_field(ev, "path"),
        str_field(task, "primary_journey_id")
    );
    for path in array_strings(task, "ui_evidence_paths") {
        text.push(' ');
        text.push_str(&path);
    }
    if str_field(ev, "digest") == digest::ZERO
        || ["fixture", "mock", "dummy", "placeholder"]
            .iter()
            .any(|token| text.to_lowercase().contains(token))
    {
        out.push(fail(
            "product_cohesion_placeholder_or_mock_proof",
            str_field(ev, "id"),
        ));
    }
    let required_ui = array_strings(task, "ui_evidence_paths")
        .into_iter()
        .collect::<BTreeSet<_>>();
    if !required_ui.is_empty() && !required_ui.iter().all(|p| ui_paths.contains(p.as_str())) {
        out.push(fail(
            "product_ui_journey_evidence_mismatch",
            str_field(ev, "id"),
        ));
    }
    human_attention_exception_check(ev, task, evidence_paths, out);
    let exhausted = array_strings(task, "exhausted_harness_evidence_paths");
    if exhausted.is_empty()
        || !exhausted
            .iter()
            .all(|p| evidence_paths.contains(p.as_str()))
    {
        out.push(fail(
            "product_harness_paths_not_exhausted",
            str_field(ev, "id"),
        ));
    }
}

fn human_attention_exception_check(
    ev: &Value,
    task: &Value,
    evidence_paths: &BTreeSet<&str>,
    out: &mut Vec<Failure>,
) {
    if !["frequent", "unknown"].contains(&str_field(task, "expected_interruption_rate").as_str()) {
        return;
    }
    let exception_path = task
        .pointer("/human_review_queue_exception/evidence_path")
        .and_then(Value::as_str);
    if let Some(exception_path) = exception_path {
        if !evidence_paths.contains(exception_path) {
            out.push(fail(
                "product_human_attention_exception_evidence_missing",
                str_field(ev, "id"),
            ));
        }
    } else {
        out.push(fail("product_human_attention_overuse", str_field(ev, "id")));
    }
}

fn fail(error: &str, detail: impl Into<String>) -> Failure {
    Failure::new("product-cohesion-proof", error, detail)
}

fn str_field(value: &Value, key: &str) -> String {
    json_boundary::string(value, key).unwrap_or_default()
}

fn array_strings(value: &Value, key: &str) -> Vec<String> {
    json_boundary::string_array(value, key)
}
