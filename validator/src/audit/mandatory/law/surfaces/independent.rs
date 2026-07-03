use serde_json::Value;
use std::path::Path;

pub(super) fn verification_failures(
    root: &Path,
    value: &Value,
    law: &str,
    current_digest: &str,
) -> Vec<String> {
    let Some(verification) = value.get("independent_verification") else {
        return if requires_independent_verification(law) {
            vec![format!(
                "mandatory_law_independent_verification_missing:{law}"
            )]
        } else {
            Vec::new()
        };
    };
    let mut out = shape_failures(verification, law);
    let Some(receipt_path) = verification.get("receipt_path").and_then(Value::as_str) else {
        out.push(format!(
            "mandatory_law_independent_verification_receipt_missing:{law}"
        ));
        return out;
    };
    if !safe_manual_receipt_path(receipt_path) {
        out.push(format!(
            "mandatory_law_independent_verification_receipt_path_invalid:{law}"
        ));
        return out;
    }
    match crate::json_boundary::read_json(&root.join(receipt_path)) {
        Ok(receipt) => out.extend(receipt_failures(&receipt, law, current_digest)),
        Err(_) => out.push(format!(
            "mandatory_law_independent_verification_receipt_missing:{law}"
        )),
    }
    out
}

fn requires_independent_verification(law: &str) -> bool {
    law == "validator-theater-miswire-resistance"
}

fn shape_failures(verification: &Value, law: &str) -> Vec<String> {
    let mut out = Vec::new();
    if verification.get("authority").and_then(Value::as_str)
        != Some("parent_verified_source_runtime")
    {
        out.push(format!(
            "mandatory_law_independent_verification_authority_invalid:{law}"
        ));
    }
    if verification
        .get("cli_pass_alone_allowed")
        .and_then(Value::as_bool)
        != Some(false)
    {
        out.push(format!(
            "mandatory_law_independent_verification_cli_only:{law}"
        ));
    }
    if verification
        .get("source_runtime_manual_required")
        .and_then(Value::as_bool)
        != Some(true)
    {
        out.push(format!(
            "mandatory_law_independent_verification_manual_required_missing:{law}"
        ));
    }
    if !matches!(
        verification
            .get("verification_scope")
            .and_then(Value::as_str),
        Some("per_law" | "per_claim_path" | "full_contract")
    ) {
        out.push(format!(
            "mandatory_law_independent_verification_scope_invalid:{law}"
        ));
    }
    out
}

fn safe_manual_receipt_path(path: &str) -> bool {
    !path.starts_with('/')
        && !path.contains("..")
        && path.starts_with("validation_artifacts/manual/")
        && path.ends_with(".json")
}

fn receipt_failures(receipt: &Value, law: &str, current_digest: &str) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.parent-source-runtime-verification.v1")
    {
        out.push(format!(
            "mandatory_law_independent_verification_receipt_wrong_schema:{law}"
        ));
    }
    if receipt.get("status").and_then(Value::as_str) != Some("verified_current") {
        out.push(format!(
            "mandatory_law_independent_verification_receipt_not_current:{law}"
        ));
    }
    if receipt.get("candidate_digest").and_then(Value::as_str) != Some(current_digest) {
        out.push(format!(
            "mandatory_law_independent_verification_receipt_digest_mismatch:{law}"
        ));
    }
    if receipt
        .get("cli_pass_alone_rejected")
        .and_then(Value::as_bool)
        != Some(true)
    {
        out.push(format!(
            "mandatory_law_independent_verification_receipt_cli_only:{law}"
        ));
    }
    if !receipt_covers_law(receipt, law) {
        out.push(format!(
            "mandatory_law_independent_verification_receipt_law_missing:{law}"
        ));
    }
    for field in ["source_paths", "runtime_evidence_paths", "manual_checks"] {
        if receipt
            .get(field)
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            out.push(format!(
                "mandatory_law_independent_verification_receipt_missing_field:{law}:{field}"
            ));
        }
    }
    out
}

fn receipt_covers_law(receipt: &Value, law: &str) -> bool {
    receipt
        .get("law_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|id| id == law)
}
