use crate::audit::contract::Failure;
use serde_json::Value;

pub(crate) fn shape(claim: &Value, receipt: &Value, ev: &Value, out: &mut Vec<Failure>) {
    if str_field(receipt, "schema") != "harness-ultragoal.coverage-receipt.v1"
        || str_field(receipt, "command").is_empty()
        || str_field(receipt, "tool").is_empty()
    {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_receipt_malformed",
            str_field(ev, "id"),
        ));
    }
    if str_field(receipt, "claim_id") != str_field(claim, "id") {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_receipt_wrong_claim_id",
            format!(
                "receipt={} claim={}",
                str_field(receipt, "claim_id"),
                str_field(claim, "id")
            ),
        ));
    }
    if receipt
        .get("target_paths")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_claim_missing_target_paths",
            str_field(ev, "id"),
        ));
    }
    if receipt
        .get("measured_dimensions")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_claim_missing_dimensions",
            str_field(ev, "id"),
        ));
    }
    ratchet_shape(receipt, ev, out);
}

pub(crate) fn percent(receipt: &Value, text: &str, out: &mut Vec<Failure>) {
    let percent = receipt
        .pointer("/coverage/percent")
        .and_then(Value::as_f64)
        .unwrap_or(-1.0);
    let policy = receipt
        .pointer("/coverage/policy")
        .and_then(Value::as_str)
        .unwrap_or("");
    let uncovered = receipt
        .get("uncovered_records")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if text.contains("100") && (percent < 100.0 || uncovered > 0) {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_claim_uncovered_code",
            format!("percent={percent} uncovered={uncovered}"),
        ));
    }
    if completion_claim(text) && (percent < 100.0 || policy != "100_percent_required") {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_ratchet_presented_as_complete",
            format!("percent={percent} policy={policy}"),
        ));
    }
}

pub(crate) fn dimensions(receipt: &Value, text: &str, out: &mut Vec<Failure>) {
    let dims = receipt
        .get("measured_dimensions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let target_text = receipt
        .get("target_paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    for required in required_dimensions(text, &target_text) {
        if !dims.contains(required) {
            out.push(Failure::new(
                "coverage-proof-policy",
                "coverage_required_dimension_missing",
                required,
            ));
        }
    }
}

fn ratchet_shape(receipt: &Value, ev: &Value, out: &mut Vec<Failure>) {
    let coverage = receipt.get("coverage").unwrap_or(&Value::Null);
    if str_field(coverage, "policy") != "ratchet_floor" {
        return;
    }
    if str_field(coverage, "owner").is_empty() {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_ratchet_missing_owner",
            str_field(ev, "id"),
        ));
    }
    if str_field(coverage, "blocker_or_debt_id").is_empty() {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_ratchet_missing_debt",
            str_field(ev, "id"),
        ));
    }
}

fn required_dimensions<'a>(text: &str, target_text: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    if target_text.contains("src") || text.contains("source code") {
        out.push("line");
    }
    if any(text, &["branch", "conditional", "match arm", "error path"]) {
        out.push("branch");
    }
    if any(
        text,
        &[
            "function",
            "api",
            "command",
            "route",
            "handler",
            "entrypoint",
            "workflow step",
        ],
    ) {
        out.push("function");
    }
    if text.contains("region") {
        out.push("region");
    }
    if any(
        text,
        &[
            "user-facing",
            "ui",
            "dashboard",
            "control surface",
            "form",
            "replay view",
            "run console",
            "install flow",
            "product route",
        ],
    ) {
        out.push("ui_state");
    }
    if any(
        text,
        &[
            "artifact",
            "receipt",
            "manifest",
            "schema",
            "index",
            "archive",
            "package",
            "cache",
            "validator output",
            "derived authority",
        ],
    ) {
        out.push("artifact");
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn completion_claim(text: &str) -> bool {
    [
        "complete",
        "done",
        "ready",
        "production-ready",
        "release",
        "release-ready",
        "release advancement",
        "material sign-off",
    ]
    .iter()
    .any(|term| text.contains(term))
}

fn any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
