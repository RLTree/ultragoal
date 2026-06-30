use serde_json::{Value, json};
use std::path::Path;

pub(crate) const DEFAULT_POLICY: &str = "docs/openai-provider-policy.json";

pub(crate) struct BudgetSelection {
    pub(crate) policy_digest: String,
    pub(crate) budget_class: String,
    pub(crate) selected: Value,
    pub(crate) failures: Vec<String>,
}

pub(crate) fn default_class(provider_mode: &str) -> &'static str {
    match provider_mode {
        "offline_fixture" => "source_offline_fixture",
        "local_mock" => "source_local_mock",
        "openai_live" => "source_live_low",
        _ => "source_no_network",
    }
}

pub(crate) fn load(
    root: &Path,
    policy_path: &Path,
    budget_class: &str,
    provider_mode: &str,
) -> BudgetSelection {
    let abs = if policy_path.is_absolute() {
        policy_path.to_path_buf()
    } else {
        root.join(policy_path)
    };
    let mut failures = Vec::new();
    let policy_digest = crate::digest::file(&abs).unwrap_or_else(|err| {
        failures.push(format!("openai_provider_policy_digest_unavailable:{err}"));
        crate::digest::ZERO.to_string()
    });
    let policy = crate::json_boundary::read_json(&abs).unwrap_or_else(|err| {
        failures.push(format!("openai_provider_policy_missing_or_malformed:{err}"));
        Value::Null
    });
    validate_policy_shape(&policy, provider_mode, &mut failures);
    let selected = policy
        .pointer(&format!("/budget_classes/{budget_class}"))
        .cloned()
        .unwrap_or_else(|| {
            failures.push("openai_provider_budget_class_missing".to_string());
            Value::Null
        });
    validate_budget_class(&selected, provider_mode, &mut failures);
    BudgetSelection {
        policy_digest,
        budget_class: budget_class.to_string(),
        selected,
        failures,
    }
}

pub(crate) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if str_field(receipt, "provider_policy_path") != DEFAULT_POLICY {
        out.push("openai_call_receipt_provider_policy_path_invalid".to_string());
    }
    let expected = crate::digest::file(&root.join(DEFAULT_POLICY)).unwrap_or_default();
    if str_field(receipt, "provider_policy_digest") != expected {
        out.push("openai_call_receipt_provider_policy_digest_mismatch".to_string());
    }
    if str_field(receipt, "budget_class").is_empty() {
        out.push("openai_call_receipt_missing_budget_class".to_string());
    }
    if !valid_cost_receipt(receipt.get("token_cost_rate_limit")) {
        out.push("openai_call_receipt_cost_rate_limit_invalid".to_string());
    }
    if !valid_timeout_policy(receipt.get("timeout_retry_backoff")) {
        out.push("openai_call_receipt_timeout_retry_backoff_invalid".to_string());
    }
    if str_field(receipt, "cache_policy").is_empty() {
        out.push("openai_call_receipt_cache_policy_missing".to_string());
    }
    out
}

pub(crate) fn cost_rate_limit(
    candidate: &str,
    provider_mode: &str,
    selection: &BudgetSelection,
) -> Value {
    let status = if selection.failures.is_empty() {
        "pass"
    } else {
        "fail"
    };
    json!({
        "schema": "harness-ultragoal.openai-cost-rate-limit-receipt.v1",
        "status": status,
        "candidate_digest": candidate,
        "provider_mode": provider_mode,
        "budget_class": selection.budget_class,
        "max_prompt_tokens": number(&selection.selected, "max_prompt_tokens"),
        "max_completion_tokens": number(&selection.selected, "max_completion_tokens"),
        "max_total_tokens": number(&selection.selected, "max_total_tokens"),
        "prompt_tokens": 0,
        "completion_tokens": 0,
        "total_tokens": 0,
        "cost_ceiling_usd": decimal(&selection.selected, "cost_ceiling_usd"),
        "estimated_cost_usd": 0,
        "cost_unavailable_reason": cost_unavailable_reason(provider_mode),
        "rate_limit_observed": false,
        "rate_limit_policy": text(&selection.selected, "rate_limit_policy"),
        "rate_limit_unavailable_reason": rate_limit_unavailable_reason(provider_mode),
        "claim_impact": "cost_and_rate_limit_observation_only"
    })
}

pub(crate) fn timeout_retry_backoff(selection: &BudgetSelection) -> Value {
    json!({
        "timeout_ms": number(&selection.selected, "timeout_ms"),
        "max_retries": number(&selection.selected, "max_retries"),
        "backoff_policy": text(&selection.selected, "backoff_policy"),
        "backoff_initial_ms": number(&selection.selected, "backoff_initial_ms"),
        "backoff_max_ms": number(&selection.selected, "backoff_max_ms"),
        "cache_policy": text(&selection.selected, "cache_policy"),
        "no_cache_verification": text(&selection.selected, "no_cache_verification")
    })
}

pub(crate) fn cache_policy(selection: &BudgetSelection) -> String {
    text(&selection.selected, "cache_policy")
}

fn validate_policy_shape(policy: &Value, provider_mode: &str, failures: &mut Vec<String>) {
    if policy.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.openai-provider-policy.v1")
    {
        failures.push("openai_provider_policy_wrong_schema".to_string());
    }
    if policy.get("law_id").and_then(Value::as_str) != Some(super::LAW_ID) {
        failures.push("openai_provider_policy_wrong_law_id".to_string());
    }
    if !array_contains(policy.get("allowed_provider_modes"), provider_mode) {
        failures.push("openai_provider_mode_not_allowed_by_policy".to_string());
    }
}

fn validate_budget_class(class: &Value, provider_mode: &str, failures: &mut Vec<String>) {
    if !array_contains(class.get("provider_modes"), provider_mode) {
        failures.push("openai_provider_budget_class_mode_mismatch".to_string());
    }
    if bounded_number(class, "timeout_ms", 1000, 120000).is_none() {
        failures.push("openai_provider_budget_timeout_invalid".to_string());
    }
    if bounded_number(class, "max_retries", 0, 3).is_none() {
        failures.push("openai_provider_budget_retry_invalid".to_string());
    }
    if provider_mode != "openai_live" && number(class, "max_retries") != 0 {
        failures.push("openai_provider_non_live_retry_must_be_zero".to_string());
    }
    if text(class, "cache_policy").is_empty() || text(class, "no_cache_verification").is_empty() {
        failures.push("openai_provider_cache_policy_missing".to_string());
    }
}

fn valid_cost_receipt(value: Option<&Value>) -> bool {
    value.is_some_and(|receipt| {
        receipt.get("schema").and_then(Value::as_str)
            == Some("harness-ultragoal.openai-cost-rate-limit-receipt.v1")
            && receipt.get("status").and_then(Value::as_str) == Some("pass")
            && receipt
                .get("max_total_tokens")
                .and_then(Value::as_i64)
                .is_some()
            && receipt
                .get("cost_ceiling_usd")
                .and_then(Value::as_f64)
                .is_some()
            && receipt
                .get("rate_limit_policy")
                .and_then(Value::as_str)
                .is_some()
    })
}

fn valid_timeout_policy(value: Option<&Value>) -> bool {
    value.is_some_and(|policy| {
        let timeout = policy
            .get("timeout_ms")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let retries = policy
            .get("max_retries")
            .and_then(Value::as_i64)
            .unwrap_or(-1);
        (1000..=120000).contains(&timeout)
            && (0..=3).contains(&retries)
            && policy
                .get("backoff_policy")
                .and_then(Value::as_str)
                .is_some()
            && policy.get("cache_policy").and_then(Value::as_str).is_some()
    })
}

fn array_contains(value: Option<&Value>, expected: &str) -> bool {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|item| item.as_str() == Some(expected))
}

fn bounded_number(value: &Value, key: &str, min: i64, max: i64) -> Option<i64> {
    let found = number(value, key);
    (min..=max).contains(&found).then_some(found)
}

fn number(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn decimal(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn str_field(value: &Value, key: &str) -> String {
    text(value, key)
}

fn cost_unavailable_reason(provider_mode: &str) -> &'static str {
    match provider_mode {
        "openai_live" => "live_provider_not_invoked_by_boundary_probe",
        "no_network" => "no_provider_call",
        _ => "offline_or_mock_provider",
    }
}

fn rate_limit_unavailable_reason(provider_mode: &str) -> &'static str {
    match provider_mode {
        "openai_live" => "live_provider_not_invoked_by_boundary_probe",
        "no_network" => "no_provider_call",
        _ => "offline_or_mock_provider",
    }
}
