use super::registry;
use serde_json::Value;
use std::path::Path;

pub(super) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    registry::require_str(receipt, "schema", super::RECEIPT_SCHEMA, &mut out);
    registry::require_str(receipt, "status", "pass", &mut out);
    registry::require_str(receipt, "candidate_digest", &candidate, &mut out);
    registry::require_str(
        receipt,
        "raw_observation_authority",
        "observation_only_until_cli_parsed_loop_receipt",
        &mut out,
    );
    require_digest(root, receipt, "registry_digest", super::REGISTRY, &mut out);
    if !blocks_required_claims(receipt) {
        out.push("improvement_loop_receipt_missing_claim_blockers".to_string());
    }
    check_observability(receipt, &candidate, &mut out);
    out
}
fn require_digest(root: &Path, receipt: &Value, key: &str, rel: &str, out: &mut Vec<String>) {
    let expected =
        crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string());
    if receipt.get(key).and_then(Value::as_str) != Some(expected.as_str()) {
        out.push(format!("improvement_loop_receipt_digest_mismatch:{key}"));
    }
}
fn blocks_required_claims(receipt: &Value) -> bool {
    let claims = registry::array_strings(receipt.get("blocked_claims"));
    [
        "completion",
        "readiness",
        "release",
        "final_packet_correctness",
        "update_goal_eligibility",
        "self_improving_claim",
        "learning_claim",
        "regression_prevention_claim",
        "product_learning_claim",
        "reviewer_exposure",
        "app_registry_exposure",
    ]
    .iter()
    .all(|claim| claims.contains(claim))
}
fn check_observability(receipt: &Value, candidate: &str, out: &mut Vec<String>) {
    let _ = (receipt, candidate);
    out.push("improvement_loop_retired_observability_binding".to_string());
}
