use serde_json::Value;

pub(super) fn package_failures(package: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if package
        .pointer("/devDependencies/promptfoo")
        .and_then(Value::as_str)
        != Some(super::PROMPTFOO_VERSION)
    {
        out.push("promptfoo_package_json_not_pinned".to_string());
    }
    if package.get("packageManager").and_then(Value::as_str) != Some("pnpm@11.1.2") {
        out.push("promptfoo_package_manager_not_pinned".to_string());
    }
    out
}

pub(super) fn provider_failures(provider: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_value(
        provider,
        "/schema",
        "harness-ultragoal.promptfoo-provider-registry.v1",
        &mut out,
    );
    require_value(provider, "/law_id", super::LAW_ID, &mut out);
    for required in ["no_network", "offline_fixture", "local_mock", "openai_live"] {
        if !provider_modes(provider).iter().any(|mode| mode == required) {
            out.push(format!("promptfoo_provider_mode_missing:{required}"));
        }
    }
    for forbidden in [
        "raw_promptfoo_pass_as_claim",
        "promptfoo_eval_as_product_success",
    ] {
        if !array_strings(provider.pointer("/forbidden_substitutions")).contains(&forbidden) {
            out.push(format!(
                "promptfoo_forbidden_substitution_missing:{forbidden}"
            ));
        }
    }
    out
}

pub(super) fn suite_failures(suite: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_value(
        suite,
        "/schema",
        "harness-ultragoal.promptfoo-suite-registry.v1",
        &mut out,
    );
    require_value(suite, "/law_id", super::LAW_ID, &mut out);
    if !suite_ids(suite)
        .iter()
        .any(|id| id == "claim-ceiling-provider-separation-smoke")
    {
        out.push("promptfoo_suite_smoke_missing".to_string());
    }
    out
}

pub(super) fn provider_modes(value: &Value) -> Vec<String> {
    value
        .pointer("/providers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("mode").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

pub(super) fn suite_ids(value: &Value) -> Vec<String> {
    value
        .pointer("/suites")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

pub(super) fn require_value(value: &Value, pointer: &str, expected: &str, out: &mut Vec<String>) {
    if value.pointer(pointer).and_then(Value::as_str) != Some(expected) {
        out.push(format!("promptfoo_registry_field_mismatch:{pointer}"));
    }
}

pub(super) fn require_str(receipt: &Value, key: &str, expected: &str, out: &mut Vec<String>) {
    if receipt.get(key).and_then(Value::as_str) != Some(expected) {
        out.push(format!("promptfoo_receipt_field_mismatch:{key}"));
    }
}

pub(super) fn array_strings(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}
