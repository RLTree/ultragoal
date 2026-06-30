use serde_json::Value;
use std::path::Path;

const POLICY_REL: &str = "docs/openai-key-policy.json";
const RECEIPT_REL: &str = "validation_artifacts/openai/config-receipt.json";
const SCHEMA: &str = "harness-ultragoal.openai-config-receipt.v1";

pub(crate) fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    check_policy(root, &mut out);
    check_receipt(root, &mut out);
    out
}

fn check_policy(root: &Path, out: &mut Vec<String>) {
    let policy = crate::cli::openai::policy::load(root, Path::new(POLICY_REL));
    out.extend(policy.failures());
}

fn check_receipt(root: &Path, out: &mut Vec<String>) {
    let candidate = match crate::package::inventory::package_digest(root) {
        Ok(candidate) => candidate,
        Err(err) => {
            out.push(format!("openai_candidate_digest_unavailable:{err}"));
            return;
        }
    };
    let receipt = match crate::json_boundary::read_json(&root.join(RECEIPT_REL)) {
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
        || crate::cli::openai::policy::contains_secret_shape(&receipt)
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
    check_observability_binding(&receipt, &candidate, out);
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

fn check_observability_binding(receipt: &Value, candidate: &str, out: &mut Vec<String>) {
    let Some(obs) = receipt.get("observability_receipt") else {
        out.push("openai_config_receipt_missing_observability_binding".to_string());
        return;
    };
    if obs.get("schema").and_then(Value::as_str) != Some(crate::cli::observe::types::RECEIPT_SCHEMA)
        || obs.get("status").and_then(Value::as_str) != Some("pass")
        || obs.get("candidate_digest").and_then(Value::as_str) != Some(candidate)
        || obs.get("law_id").and_then(Value::as_str) != Some(crate::cli::openai::LAW_ID)
        || obs.get("check_id").and_then(Value::as_str) != Some(crate::cli::openai::LAW_ID)
    {
        out.push("openai_config_receipt_observability_binding_invalid".to_string());
    }
}
