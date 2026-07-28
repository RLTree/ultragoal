use serde_json::Value;
use std::path::Path;

const PACKAGE_JSON: &str = "package.json";
const PNPM_LOCK: &str = "pnpm-lock.yaml";
const PNPM_WORKSPACE: &str = "pnpm-workspace.yaml";
const PROVIDER_REGISTRY: &str = "docs/promptfoo-provider-registry.json";
const SUITE_REGISTRY: &str = "docs/promptfoo-suite-registry.json";

pub(super) fn failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    require_str(receipt, "schema", super::SCHEMA, &mut out);
    require_str(receipt, "status", "pass", &mut out);
    require_str(receipt, "candidate_digest", &candidate, &mut out);
    require_str(
        receipt,
        "promptfoo_version",
        super::PROMPTFOO_VERSION,
        &mut out,
    );
    require_str(
        receipt,
        "raw_promptfoo_authority",
        "observation_only_until_cli_parsed_receipt",
        &mut out,
    );
    for (key, rel) in [
        ("package_json_digest", PACKAGE_JSON),
        ("pnpm_lock_digest", PNPM_LOCK),
        ("pnpm_workspace_digest", PNPM_WORKSPACE),
        ("provider_registry_digest", PROVIDER_REGISTRY),
        ("suite_registry_digest", SUITE_REGISTRY),
    ] {
        require_digest(root, receipt, key, rel, &mut out);
    }
    if !blocks_required_claims(receipt) {
        out.push("promptfoo_receipt_missing_claim_blockers".to_string());
    }
    out.push("promptfoo_retired_observability_binding".to_string());
    if crate::audit::openai::contains_secret_shape(receipt) {
        out.push("promptfoo_receipt_secret_shape_detected".to_string());
    }
    out
}

fn require_str(receipt: &Value, key: &str, expected: &str, out: &mut Vec<String>) {
    if receipt.get(key).and_then(Value::as_str) != Some(expected) {
        out.push(format!("promptfoo_receipt_field_mismatch:{key}"));
    }
}

fn require_digest(root: &Path, receipt: &Value, key: &str, rel: &str, out: &mut Vec<String>) {
    let expected =
        crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string());
    if receipt.get(key).and_then(Value::as_str) != Some(expected.as_str()) {
        out.push(format!("promptfoo_receipt_digest_mismatch:{key}"));
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
        "product_fitness",
        "live_model_authority",
        "promptfoo_raw_pass_authority",
        "reviewer_exposure",
        "app_registry_exposure",
        "eval_result_claim",
    ]
    .iter()
    .all(|claim| claims.contains(claim))
}
