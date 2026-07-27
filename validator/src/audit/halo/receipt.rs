use serde_json::Value;
use std::path::Path;

pub(super) fn failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    require_str(receipt, "schema", super::SCHEMA, &mut out);
    require_str(receipt, "status", "pass", &mut out);
    require_str(receipt, "candidate_digest", &candidate, &mut out);
    require_str(
        receipt,
        "raw_halo_authority",
        "observation_only_until_cli_adapter_ranking_and_validation",
        &mut out,
    );
    require_digest(
        root,
        receipt,
        "adapter_registry_digest",
        super::REGISTRY,
        &mut out,
    );
    if !blocks_required_claims(receipt) {
        out.push("halo_capability_receipt_missing_claim_blockers".to_string());
    }
    if !matches!(
        receipt.get("authority_class").and_then(Value::as_str),
        Some("manual_observation_only" | "fail_closed")
    ) {
        out.push("halo_capability_authority_overbroad".to_string());
    }
    out.push("halo_capability_retired_observability_binding".to_string());
    out
}

fn require_str(receipt: &Value, key: &str, expected: &str, out: &mut Vec<String>) {
    if receipt.get(key).and_then(Value::as_str) != Some(expected) {
        out.push(format!("halo_capability_receipt_field_mismatch:{key}"));
    }
}

fn require_digest(root: &Path, receipt: &Value, key: &str, rel: &str, out: &mut Vec<String>) {
    let expected =
        crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string());
    if receipt.get(key).and_then(Value::as_str) != Some(expected.as_str()) {
        out.push(format!("halo_capability_receipt_digest_mismatch:{key}"));
    }
}

fn blocks_required_claims(receipt: &Value) -> bool {
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
        "product_success",
        "registry_exposure",
        "reviewer_exposure",
        "halo_ranked_change_authority",
        "halo_recommendation_readiness",
        "improvement_loop_closure",
    ]
    .iter()
    .all(|claim| claims.contains(claim))
}
