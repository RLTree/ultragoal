use serde_json::Value;
use std::path::Path;

#[path = "output/receipt.rs"]
mod output_receipt;
mod policy;
#[path = "provider/policy.rs"]
mod provider_policy;

const POLICY_REL: &str = "docs/openai-key-policy.json";
const RECEIPT_REL: &str = "validation_artifacts/openai/config-receipt.json";
const CALL_RECEIPT_REL: &str = "validation_artifacts/openai/call-receipt.json";
const OUTPUT_RECEIPT_REL: &str = "validation_artifacts/openai/model-output-authority.json";
const SCHEMA: &str = "harness-ultragoal.openai-config-receipt.v1";
const CALL_SCHEMA: &str = "harness-ultragoal.openai-call-receipt.v1";
const PROVIDER_POLICY_REL: &str = "docs/openai-provider-policy.json";
pub(crate) const LAW_ID: &str = "openai-api-key-model-cost-external-ai-boundary";

pub(crate) use policy::contains_secret_shape;

#[cfg(test)]
mod tests;

pub(crate) fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    check_policy(root, &mut out);
    check_provider_policy(root, &mut out);
    check_receipt(root, &mut out);
    check_call_receipt(root, &mut out);
    check_output_receipt(root, &mut out);
    out
}

fn check_policy(root: &Path, out: &mut Vec<String>) {
    let policy = policy::load(root, Path::new(POLICY_REL));
    out.extend(policy.failures());
}

fn candidate_or_empty(root: &Path) -> String {
    crate::package::inventory::package_digest(root).unwrap_or_default()
}

fn check_provider_policy(root: &Path, out: &mut Vec<String>) {
    let failures = provider_policy::failures(
        root,
        Path::new(PROVIDER_POLICY_REL),
        "source_no_network",
        "no_network",
    );
    out.extend(failures);
}

fn check_call_receipt(root: &Path, out: &mut Vec<String>) {
    let candidate = match crate::package::inventory::package_digest(root) {
        Ok(candidate) => candidate,
        Err(err) => {
            out.push(format!("openai_call_candidate_digest_unavailable:{err}"));
            return;
        }
    };
    let receipt_path = crate::output_path::literal_claim_artifact_path(
        root,
        CALL_RECEIPT_REL,
        "OpenAI call receipt",
    );
    let receipt = match crate::json_boundary::read_json(&receipt_path) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("openai_call_receipt_missing_or_malformed:{err}"));
            return;
        }
    };
    if receipt.get("schema").and_then(Value::as_str) != Some(CALL_SCHEMA) {
        out.push("openai_call_receipt_wrong_schema".to_string());
    }
    if receipt.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("openai_call_receipt_not_passing".to_string());
    }
    if receipt.get("candidate_digest").and_then(Value::as_str) != Some(candidate.as_str()) {
        out.push("openai_call_receipt_candidate_digest_mismatch".to_string());
    }
    if receipt.get("redaction_status").and_then(Value::as_str) != Some("pass")
        || contains_secret_shape(&receipt)
    {
        out.push("openai_call_receipt_secret_leak_or_redaction_failure".to_string());
    }
    if receipt
        .get("model_output_authority")
        .and_then(Value::as_str)
        != Some("observation_only_until_cli_schema_validated")
    {
        out.push("openai_call_receipt_model_output_authority_overbroad".to_string());
    }
    if !valid_digest(&receipt, "prompt_input_digest") || !valid_digest(&receipt, "output_digest") {
        out.push("openai_call_receipt_digest_fields_invalid".to_string());
    }
    if !blocks_completion_claims(&receipt) {
        out.push("openai_call_receipt_missing_completion_blockers".to_string());
    }
    out.extend(provider_policy::receipt_failures(root, &receipt));
    check_observability_binding(&receipt, &candidate, "openai_call", out);
}

fn check_output_receipt(root: &Path, out: &mut Vec<String>) {
    let receipt_path = crate::output_path::literal_claim_artifact_path(
        root,
        OUTPUT_RECEIPT_REL,
        "OpenAI model output authority receipt",
    );
    let receipt = match crate::json_boundary::read_json(&receipt_path) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!(
                "openai_model_output_receipt_missing_or_malformed:{err}"
            ));
            return;
        }
    };
    out.extend(output_receipt::failures(root, &receipt));
    check_observability_binding(
        &receipt,
        &candidate_or_empty(root),
        "openai_model_output",
        out,
    );
}

fn check_receipt(root: &Path, out: &mut Vec<String>) {
    let candidate = match crate::package::inventory::package_digest(root) {
        Ok(candidate) => candidate,
        Err(err) => {
            out.push(format!("openai_candidate_digest_unavailable:{err}"));
            return;
        }
    };
    let receipt_path =
        crate::output_path::literal_claim_artifact_path(root, RECEIPT_REL, "OpenAI config receipt");
    let receipt = match crate::json_boundary::read_json(&receipt_path) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("openai_config_receipt_missing_or_malformed:{err}"));
            return;
        }
    };
    if receipt.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push("openai_config_receipt_wrong_schema".to_string());
    }
    if receipt.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("openai_config_receipt_not_passing".to_string());
    }
    if receipt.get("candidate_digest").and_then(Value::as_str) != Some(candidate.as_str()) {
        out.push("openai_config_receipt_candidate_digest_mismatch".to_string());
    }
    if receipt.get("redaction_status").and_then(Value::as_str) != Some("pass")
        || contains_secret_shape(&receipt)
    {
        out.push("openai_config_receipt_secret_leak_or_redaction_failure".to_string());
    }
    if receipt
        .get("secret_material_serialized")
        .and_then(Value::as_bool)
        != Some(false)
    {
        out.push("openai_config_receipt_secret_material_serialized".to_string());
    }
    if receipt.get("claim_ceiling").and_then(Value::as_str) != Some("openai_config_resolution_only")
    {
        out.push("openai_config_receipt_claim_ceiling_overbroad".to_string());
    }
    if !blocks_completion_claims(&receipt) {
        out.push("openai_config_receipt_missing_completion_blockers".to_string());
    }
    check_observability_binding(&receipt, &candidate, "openai_config", out);
}

fn blocks_completion_claims(receipt: &Value) -> bool {
    let claims = receipt
        .get("blocked_claims")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    [
        "completion",
        "readiness",
        "release",
        "final_packet_correctness",
        "update_goal_eligibility",
        "model_output_authority",
    ]
    .into_iter()
    .all(|claim| claims.contains(&claim))
}

fn check_observability_binding(
    receipt: &Value,
    candidate: &str,
    prefix: &str,
    out: &mut Vec<String>,
) {
    let Some(obs) = receipt.get("observability_receipt") else {
        out.push(format!("{prefix}_receipt_missing_observability_binding"));
        return;
    };
    if obs.get("schema").and_then(Value::as_str)
        != Some(crate::audit::observability::RECEIPT_SCHEMA)
        || obs.get("status").and_then(Value::as_str) != Some("pass")
        || obs.get("candidate_digest").and_then(Value::as_str) != Some(candidate)
        || obs.get("law_id").and_then(Value::as_str) != Some(LAW_ID)
        || obs.get("check_id").and_then(Value::as_str) != Some(LAW_ID)
    {
        out.push(format!("{prefix}_receipt_observability_binding_invalid"));
    }
}

fn valid_digest(receipt: &Value, key: &str) -> bool {
    receipt
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| value.strip_prefix("sha256:"))
        .is_some_and(|tail| tail.len() == 64 && tail.chars().all(|ch| ch.is_ascii_hexdigit()))
}
