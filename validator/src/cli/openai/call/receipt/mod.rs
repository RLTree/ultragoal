use super::CallCommand;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn live_observation(
    root: &Path,
    command: &CallCommand,
    budget: &crate::cli::openai::budget::BudgetSelection,
) -> Result<Option<crate::cli::openai::live::Observation>, String> {
    if command.provider_mode == "openai_live" {
        crate::cli::openai::live::execute(root, command, budget).map(Some)
    } else {
        Ok(None)
    }
}

pub(super) fn prompt_input_digest(
    command: &CallCommand,
    live: Option<&crate::cli::openai::live::Observation>,
) -> String {
    live.map(|observation| observation.prompt_input_digest.clone())
        .unwrap_or_else(|| command.input_digest.clone())
}

pub(super) fn output_digest(
    command: &CallCommand,
    live: Option<&crate::cli::openai::live::Observation>,
) -> String {
    live.map(|observation| observation.output_digest.clone())
        .unwrap_or_else(|| command.output_digest.clone())
}

pub(super) fn receipt_failures(
    input_digest: &str,
    output_digest: &str,
    budget: &crate::cli::openai::budget::BudgetSelection,
) -> Vec<String> {
    let mut failures = Vec::new();
    if !is_sha256(input_digest) {
        failures.push("openai_call_prompt_input_digest_invalid".to_string());
    }
    if !is_sha256(output_digest) {
        failures.push("openai_call_output_digest_invalid".to_string());
    }
    failures.extend(budget.failures.clone());
    failures
}

pub(super) fn request_id(
    command: &CallCommand,
    live: Option<&crate::cli::openai::live::Observation>,
) -> String {
    if let Some(observation) = live {
        return observation.request_id.clone();
    }
    match command.provider_mode.as_str() {
        "no_network" => "not_available:no_network".to_string(),
        "offline_fixture" => "not_available:offline_fixture".to_string(),
        "openai_live" => "not_available:openai_live_not_invoked".to_string(),
        _ => "not_available:local_mock".to_string(),
    }
}

pub(super) fn token_cost_rate_limit(
    candidate: &str,
    command: &CallCommand,
    budget: &crate::cli::openai::budget::BudgetSelection,
    live: Option<&crate::cli::openai::live::Observation>,
) -> Value {
    let mut receipt =
        crate::cli::openai::budget::cost_rate_limit(candidate, &command.provider_mode, budget);
    if let Some(observation) = live {
        receipt["prompt_tokens"] = json!(observation.prompt_tokens);
        receipt["completion_tokens"] = json!(observation.completion_tokens);
        receipt["total_tokens"] = json!(observation.total_tokens);
        receipt["estimated_cost_usd"] = json!(0);
        receipt["cost_unavailable_reason"] =
            json!("pricing_table_not_embedded_live_usage_recorded");
        receipt["rate_limit_observed"] = json!(observation.rate_limit_observed);
        receipt["rate_limit_unavailable_reason"] = if observation.rate_limit_observed {
            json!("none")
        } else {
            json!("rate_limit_headers_not_returned")
        };
        receipt["latency_ms"] = json!(observation.latency_ms);
    }
    receipt
}

pub(super) fn secret_leak_receipt(mut value: Value) -> Value {
    value["status"] = json!("fail");
    value["redaction_status"] = json!("fail");
    value["supported_claims"] = json!([]);
    value["claim_impact"] = json!("withheld_or_blocked");
    value["failures"] = json!(["openai_call_receipt_secret_shape_detected"]);
    value
}

pub(super) fn print_receipt(receipt: &Path, value: &Value) {
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail");
    println!(
        "ultragoal-openai-call {status} candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        value
            .get("candidate_digest")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        receipt.display(),
        value
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("correlation_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("claim_impact")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        crate::cli::openai::csv(value.get("supported_claims")),
        crate::cli::openai::csv(value.get("blocked_claims"))
    );
}

fn is_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|tail| tail.len() == 64 && tail.chars().all(|ch| ch.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests;
