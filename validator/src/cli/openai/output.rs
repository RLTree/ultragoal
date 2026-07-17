use serde_json::Value;
use std::path::Path;

pub(crate) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.openai-model-output-authority.v1")
    {
        out.push("openai_model_output_receipt_wrong_schema".to_string());
    }
    if receipt.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("openai_model_output_receipt_not_passing".to_string());
    }
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    if receipt.get("candidate_digest").and_then(Value::as_str) != Some(candidate.as_str()) {
        out.push("openai_model_output_receipt_candidate_digest_mismatch".to_string());
    }
    if !valid_digest(receipt, "source_call_receipt_digest")
        || !valid_digest(receipt, "parsed_output_digest")
    {
        out.push("openai_model_output_receipt_digest_invalid".to_string());
    }
    if receipt.get("authority_state").and_then(Value::as_str)
        != Some("typed_observation_not_claim_authority")
    {
        out.push("openai_model_output_receipt_authority_overbroad".to_string());
    }
    if super::policy::contains_secret_shape(receipt)
        || receipt.get("redaction_status").and_then(Value::as_str) != Some("pass")
    {
        out.push("openai_model_output_receipt_secret_leak_or_redaction_failure".to_string());
    }
    out
}

fn valid_digest(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|digest| digest.starts_with("sha256:") && digest.len() == 71)
}
