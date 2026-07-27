use serde_json::Value;
use std::path::Path;

const DEFAULT_POLICY: &str = "docs/openai-provider-policy.json";

pub(super) fn failures(
    root: &Path,
    policy_path: &Path,
    budget_class: &str,
    provider_mode: &str,
) -> Vec<String> {
    let abs = if policy_path.is_absolute() {
        policy_path.to_path_buf()
    } else {
        root.join(policy_path)
    };
    let mut failures = Vec::new();
    if let Err(err) = crate::digest::file(&abs) {
        failures.push(format!("openai_provider_policy_digest_unavailable:{err}"));
    }
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
    failures
}

pub(super) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
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
