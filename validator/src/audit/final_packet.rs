use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::path::Path;

const RECEIPT: &str = "validation_artifacts/review/final-packet-proof.json";
const SCHEMA: &str = "final-packet-proof.schema.json";

mod references;

pub(crate) fn package_failures(root: &Path, store: &schema_catalog::SchemaStore) -> Vec<String> {
    let mut out = Vec::new();
    let receipt = match json_boundary::read_json(&root.join(RECEIPT)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("final_packet_proof_missing:{err}"));
            return out;
        }
    };
    value_failures(root, store, &receipt)
}

pub(crate) fn claim_guard_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
) -> Vec<String> {
    let mut out = Vec::new();
    let receipt = match json_boundary::read_json(&root.join(RECEIPT)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("final_packet_proof_missing:{err}"));
            return out;
        }
    };
    value_claim_guard_failures(root, store, &receipt)
}

pub(crate) fn value_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(
        schema_catalog::schema_errors(store, SCHEMA, &receipt)
            .into_iter()
            .map(|err| format!("final_packet_proof_schema:{err}")),
    );
    current_candidate_failures(root, &receipt, &mut out);
    packet_artifact_failures(root, &receipt, &mut out);
    out.extend(references::failures(root, store, &receipt));
    out
}

pub(crate) fn value_claim_guard_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
) -> Vec<String> {
    if receipt.get("status").and_then(Value::as_str) == Some("pass") {
        return value_failures(root, store, receipt);
    }
    let mut out = Vec::new();
    out.extend(
        schema_catalog::schema_errors(store, SCHEMA, &receipt)
            .into_iter()
            .map(|err| format!("final_packet_proof_schema:{err}")),
    );
    current_candidate_guard_failures(root, receipt, &mut out);
    out.extend(references::claim_guard_failures(root, store, receipt));
    out
}

fn current_candidate_failures(root: &Path, receipt: &Value, out: &mut Vec<String>) {
    let expected = crate::package::inventory::package_digest(root).unwrap_or_default();
    let actual = receipt
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        .unwrap_or("");
    if actual != expected {
        out.push(format!(
            "final_packet_proof_target_digest_mismatch:{actual}!={expected}"
        ));
    }
    if receipt.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("final_packet_proof_status_not_pass".to_string());
    }
    if receipt.get("claim_ceiling").and_then(Value::as_str)
        != Some("final_packet_evidence_dereferenced")
    {
        out.push("final_packet_proof_claim_ceiling_not_verified".to_string());
    }
    if receipt
        .pointer("/cli_performance/status")
        .and_then(Value::as_str)
        != Some("pass")
    {
        out.push("final_packet_proof_cli_performance_not_pass".to_string());
    }
}

fn current_candidate_guard_failures(root: &Path, receipt: &Value, out: &mut Vec<String>) {
    let expected = crate::package::inventory::package_digest(root).unwrap_or_default();
    let actual = receipt
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        .unwrap_or("");
    if actual != expected {
        out.push(format!(
            "final_packet_proof_target_digest_mismatch:{actual}!={expected}"
        ));
    }
    if receipt.get("status").and_then(Value::as_str) != Some("fail") {
        out.push("final_packet_proof_guard_status_not_fail".to_string());
    }
    if receipt.get("claim_ceiling").and_then(Value::as_str) != Some("withheld_or_blocked") {
        out.push("final_packet_proof_guard_claim_ceiling_not_blocking".to_string());
    }
    for claim in [
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "update_goal_eligibility",
    ] {
        if !blocked_claims_contain(receipt, claim) {
            out.push(format!(
                "final_packet_proof_guard_missing_blocked_claim:{claim}"
            ));
        }
    }
    if receipt.pointer("/failure/reason").and_then(Value::as_str)
        != Some("final_packet_proof_not_proven")
    {
        out.push("final_packet_proof_guard_failure_reason_missing".to_string());
    }
    if !receipt
        .pointer("/failure/observed_failures")
        .and_then(Value::as_array)
        .is_some_and(|failures| !failures.is_empty())
    {
        out.push("final_packet_proof_guard_observed_failures_missing".to_string());
    }
}

fn packet_artifact_failures(root: &Path, receipt: &Value, out: &mut Vec<String>) {
    let rel = receipt
        .pointer("/packet/path")
        .and_then(Value::as_str)
        .unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        out.push(format!("final_packet_proof_packet_path_invalid:{rel}"));
        return;
    }
    let got = crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.into());
    let want = receipt
        .pointer("/packet/digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if got != want {
        out.push(format!("final_packet_proof_packet_digest_mismatch:{rel}"));
    }
}

fn blocked_claims_contain(value: &Value, claim: &str) -> bool {
    value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(|claims| claims.iter().any(|item| item.as_str() == Some(claim)))
}
