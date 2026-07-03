pub(crate) mod stdout;
pub(crate) mod telemetry;

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
    let status = typed_status(root, rel).unwrap_or("fail");
    json!({"path": rel, "digest": digest, "status": status})
}

fn typed_status(root: &Path, rel: &str) -> Option<&'static str> {
    let value = crate::json_boundary::read_json(&root.join(rel)).ok()?;
    if value.get("status").and_then(Value::as_str) == Some("pass") {
        return Some("pass");
    }
    if value.get("status").and_then(Value::as_str) == Some("fail") {
        return Some("fail");
    }
    if rel == "validation_artifacts/coverage/coverage-receipt.json"
        && value.get("schema").and_then(Value::as_str)
            == Some("harness-ultragoal.coverage-receipt.v1")
        && value.get("command_exit").and_then(Value::as_i64) == Some(0)
        && value.pointer("/coverage/percent").and_then(Value::as_f64) == Some(100.0)
        && value
            .get("uncovered_records")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && value.get("claim_ceiling").and_then(Value::as_str)
            == Some("supports_complete_coverage_claim")
        && array_contains(&value, "supported_claim_classes", "complete_coverage")
    {
        return Some("pass");
    }
    None
}

fn array_contains(value: &Value, key: &str, needle: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(needle)))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn transactional_status_accepts_exact_coverage_receipt_without_status_field() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "transactional-coverage-status",
        );
        let rel = "validation_artifacts/coverage/coverage-receipt.json";
        crate::json_boundary::write_json(
            &root.join(rel),
            &json!({
                "schema":"harness-ultragoal.coverage-receipt.v1",
                "command_exit":0,
                "coverage":{"percent":100.0},
                "uncovered_records":[],
                "claim_ceiling":"supports_complete_coverage_claim",
                "supported_claim_classes":["complete_coverage"],
                "blocked_claim_classes":["completion"]
            }),
        )
        .expect("coverage receipt");
        assert_eq!(super::typed_status(&root, rel), Some("pass"));
        std::fs::remove_dir_all(root).expect("cleanup transactional coverage");
    }

    #[test]
    fn transactional_status_rejects_below_floor_coverage_receipt() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "transactional-coverage-low",
        );
        let rel = "validation_artifacts/coverage/coverage-receipt.json";
        crate::json_boundary::write_json(
            &root.join(rel),
            &json!({
                "schema":"harness-ultragoal.coverage-receipt.v1",
                "command_exit":0,
                "coverage":{"percent":99.0},
                "uncovered_records":[],
                "claim_ceiling":"withheld_or_blocked"
            }),
        )
        .expect("coverage receipt");
        assert_eq!(super::typed_status(&root, rel), None);
        assert_eq!(super::typed_status(&root, "missing.json"), None);
        std::fs::remove_dir_all(root).expect("cleanup transactional coverage low");
    }

    #[test]
    fn transactional_receipt_requires_package_digest_authority() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "transactional-no-manifest",
        );
        let error = super::receipt(&root).expect_err("missing package digest");
        assert!(error.contains("plugin-manifest-draft.json"), "{error}");
        let _ = std::fs::remove_dir_all(root);
    }
}
