use serde_json::{Value, json};
use std::path::Path;

const SCHEMA: &str = "harness-ultragoal.cli-transactional-finalization-receipt.v1";

const REFS: &[(&str, &str)] = &[
    (
        "final_packet",
        "validation_artifacts/review/final-packet-proof.json",
    ),
    (
        "registry_exposure",
        "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
    ),
    (
        "cli_performance",
        "validation_artifacts/cli/performance-receipt.json",
    ),
    (
        "coverage",
        "validation_artifacts/coverage/coverage-receipt.json",
    ),
];

pub(crate) fn receipt(root: &Path) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let store = crate::schema_catalog::load(root);
    let mut value = pass_shaped(root, &candidate);
    let failures = super::proof::transaction::receipt_failures(root, &store, &value, &candidate);
    if failures.is_empty() {
        Ok(value)
    } else {
        value["status"] = json!("fail");
        value["claim_ceiling"] = json!("withheld_or_blocked");
        value["blocked_claim_classes"] = json!([
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "update_goal_eligibility"
        ]);
        value["failure"] = json!({
            "reason": "transactional_finalization_not_proven",
            "observed_failures": failures
        });
        Ok(value)
    }
}

fn pass_shaped(root: &Path, candidate: &str) -> Value {
    let mut value = json!({
        "schema": SCHEMA,
        "generated_at": crate::audit::clock::now_iso(),
        "candidate_digest": candidate,
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "transaction_mode": "same_candidate_atomic_finalization",
        "blocked_claim_classes": [],
        "failure": Value::Null,
    });
    for (label, rel) in REFS {
        value[*label] = reference(root, rel);
    }
    value
}

fn reference(root: &Path, rel: &str) -> Value {
    let path = root.join(rel);
    let digest = crate::digest::file(&path).unwrap_or_else(|_| crate::digest::ZERO.to_string());
    let status = if path.is_file() { "pass" } else { "fail" };
    json!({"path": rel, "digest": digest, "status": status})
}
